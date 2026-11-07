/*
 * ECE/CS 285 – HW2 Hero Scene: BarcelonaPavilion
 * Mies van der Rohe's Barcelona Pavilion across its pool at midday:
 * travertine, onyx, glass walls, chrome chairs, lotus plants, pebbles
 * under the water and 164 trees, 20.7 million triangles, under one sun
 * and the sky of textures/env/pavilion_sky.hdr. BarcelonaPavilion is lit
 * by hand for whitted and cook (a dome of weak spots for the sky, fills
 * for the bounce, a little ambient), 64 spp; BarcelonaPavilionSunOnly is
 * the sun and the sky alone, for kajiya, 256 spp. Halton, seed 0.
 * Model: eMirage's Barcelona Pavilion (blender.org demo files, CC-BY), its
 * sky in textures/env/pavilion_sky.hdr.
 *
 * Output: renders/<integrator>/3_HeroScenes/BarcelonaPavilion.png
 *         renders/<integrator>/3_HeroScenes/BarcelonaPavilionSunOnly.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

#include <cmath>
#include <cstdint>
#include <cstring>

/* shaders ------------------------------------------------------------ */

/* POOL_SURFACE: the water exists only for rays coming down onto it (Ng
 * faces the ray, so Ng.y > 0). A shadow ray stops at the first hit, so
 * a glass surface would shadow the whole pool floor; rays
 * from below (shadow rays, bounces toward the sun and sky) pass through. */
static RtInt POOL_SURFACE(RtShadeContext *c)
{
    return c->Ng[1] > 0;
}

/* A number in [0, 1) from the bits of a few floats (FNV-1a, then
 * MurmurHash3's finalizer to spread the bits). */
static float Hash01(const float *x, int n)
{
    uint32_t h = 2166136261u;
    for (int i = 0; i < n; i++) {
        uint32_t bits;
        std::memcpy(&bits, &x[i], sizeof bits);
        h = (h ^ bits) * 16777619u;
    }
    h ^= h >> 16;
    h *= 0x85ebca6bu;
    h ^= h >> 13;
    h *= 0xc2b2ae35u;
    h ^= h >> 16;
    return (float)(h >> 8) * (1.0f / 16777216.0f);
}

/* PEBBLE: the file colors each pebble from a ramp (slate blue, two greys,
 * a pale beige) by a random number per object, which no MTL can carry.
 * The world position, quantized to 7 cm cells (a pebble is about 5 cm),
 * picks the ramp color, and a second hash mottles it by a fifth. */
static void PEBBLE(RtShadeContext *c)
{
    const float cell[3] = {std::floor(c->P[0] / 0.07f), std::floor(c->P[1] / 0.07f), std::floor(c->P[2] / 0.07f)};
    const float t = Hash01(cell, 3);
    const float salted[4] = {cell[0], cell[1], cell[2], 7.0f};
    const float mottle = 0.9f + 0.2f * Hash01(salted, 4);
    /* the ramp: position, color */
    const float ramp[4][4] = {{0.0f, 0.05f, 0.129f, 0.242f},
                              {0.338f, 0.381f, 0.397f, 0.361f},
                              {0.659f, 0.429f, 0.44f, 0.328f},
                              {1.0f, 0.665f, 0.68f, 0.604f}};
    int k = 0;
    while (k < 2 && t > ramp[k + 1][0])
        k++;
    const float f = (t - ramp[k][0]) / (ramp[k + 1][0] - ramp[k][0]);
    /* a little darker and bluer than the ramp: the water over them, which
     * the renderer does not tint (the pool is 40 cm deep) */
    const float tint[3] = {0.38f, 0.45f, 0.55f};
    for (int i = 0; i < 3; i++)
        c->m.albedo[i] = (ramp[k][i + 1] + (ramp[k + 1][i + 1] - ramp[k][i + 1]) * f) * mottle * tint[i];
}

/* RIPPLES: the water's normal tilted by the gradient of a small Perlin
 * height field (two octaves, 25 cm and 8 cm, a few millimeters high),
 * taken by central differences in world space, so the reflection wavers
 * and the pebbles shimmer. */
static void RIPPLES(RtShadeContext *c)
{
    const float x = c->P[0], z = c->P[2];
    const double s1 = 4.0, s2 = 12.0, a1 = 0.004, a2 = 0.0015, h = 0.01;
    const double hx = (a1 * (perlinNoise((x + h) * s1, 0.37, z * s1) - perlinNoise((x - h) * s1, 0.37, z * s1)) +
                       a2 * (perlinNoise((x + h) * s2, 5.1, z * s2) - perlinNoise((x - h) * s2, 5.1, z * s2))) / (2 * h);
    const double hz = (a1 * (perlinNoise(x * s1, 0.37, (z + h) * s1) - perlinNoise(x * s1, 0.37, (z - h) * s1)) +
                       a2 * (perlinNoise(x * s2, 5.1, (z + h) * s2) - perlinNoise(x * s2, 5.1, (z - h) * s2))) / (2 * h);
    /* the water is level, so the surface's tangents are x and z and the
     * tilted normal is (-dh/dx, 1, -dh/dz) on the side N faces */
    const float sign = c->N[1] < 0 ? -1.0f : 1.0f;
    float n[3] = {(float)-hx * sign, sign, (float)-hz * sign};
    const float len = std::sqrt(n[0] * n[0] + n[1] * n[1] + n[2] * n[2]);
    for (int i = 0; i < 3; i++)
        c->N[i] = n[i] / len;
}

/* PANE: the glass walls pass shadow rays (depth < 0), so the room behind
 * them is lit by the sun and the sky as in the reference picture; camera
 * and bounce rays see the glass, its reflection and its refraction. */
static RtInt PANE(RtShadeContext *c)
{
    return c->depth < 0 ? 0 : 1;
}

/* The hook: the pool's surface and the pebbles get their shaders, the
 * glass its cutout, and the water a dimmer reflection and less of its
 * own color than the file's (the pool mirrored the sky too brightly
 * under whitted, and the water's diffuse term made it brighter than
 * under kajiya). `user` is the building's mesh, whose materials the hook
 * starts from; the instances pass none. */
typedef struct {
    const ObjMesh *mesh;
} PavilionHook;

static void PavilionMaterials(const char *material, void *user)
{
    const ObjMesh *mesh = user ? ((const PavilionHook *)user)->mesh : NULL;
    const ObjMaterial *om = NULL;
    for (int i = 0; mesh && i < mesh->nmaterials; i++)
        if (std::strcmp(mesh->materials[i].name, material) == 0)
            om = &mesh->materials[i];
    if (std::strcmp(material, "water") == 0) {
        if (om) {
            RtMaterial m = ObjMaterialToRt(*om);
            for (int i = 0; i < 3; i++) {
                m.specular[i] = 0.75f; /* the reflection at three quarters */
                m.albedo[i] *= 0.5f;   /* half the water's own color */
            }
            rtMaterial(&m);
        }
        rtSurface(RIPPLES, NULL, 0);
        rtCutout(POOL_SURFACE, NULL, 0);
    } else if (std::strcmp(material, "glass_architectural") == 0) {
        rtCutout(PANE, NULL, 0);
    } else if (std::strcmp(material, "pebbles") == 0) {
        rtSurface(PEBBLE, NULL, 0);
    }
}

/* lights -------------------------------------------------------------- */

/* A dome of weak spot lights: `count` of them spread evenly over the
 * band of elevations [lo, hi] (fractions of the vertical, -1 to 1) on a
 * sphere of radius r about `center`, each aimed at the center with a wide
 * cone, together giving a facing surface the irradiance `E`. */
static void Dome(const Vec3 &center, float r, float lo, float hi, int count, const RtColor E, unsigned seed)
{
    /* I per light: count lights at distance r, average cosine 1/2 */
    const float I = E[0] > 0 || E[1] > 0 || E[2] > 0 ? 2 * r * r / (float)count : 0;
    const float golden = 2.39996323f;
    for (int i = 0; i < count; i++) {
        const float y = lo + (hi - lo) * ((i + 0.5f) / count);
        const float ring = std::sqrt(std::max(0.0f, 1 - y * y));
        const float a = golden * i + 0.01f * (float)(seed % 97);
        const Vec3 d(ring * std::cos(a), y, ring * std::sin(a));
        const Vec3 p = center + d * r;
        RtPoint pos = {p.x, p.y, p.z};
        RtVector dir = {-d.x, -d.y, -d.z};
        RtColor Ic = {E[0] * I, E[1] * I, E[2] * I};
        rtSpotLight(pos, dir, Ic, 75, 25, 0);
    }
}

/* The roof's underside is lit by the sunlit floor's bounce, which no
 * light from outside can reach: a grid of spots just above the slab,
 * pointing up, 2 m apart, so their pools overlap into an even light
 * (the ripple is under 2% with a 60 degree cone; eight spots 6 m apart
 * varied fivefold). Each reaches 8 m, so it costs a shadow ray only
 * near it. The slots a chair stands in are left empty, and so is the
 * strip behind the travertine wall (z < -3.6, hidden by it). A spot
 * near a wall paints it, most from about a metre out, so the row
 * nearest each wall leans away from it until its cone misses the face:
 * 25 degrees for the travertine wall, and for the brown stone wall
 * (z 4.1 to 4.3, seen nearly edge-on) 30 degrees with a narrower cone,
 * a metre past it. */
static void CeilingGrid(const RtColor I)
{
    const float xs[] = {-1.0f, 1.0f, 3.0f, 5.0f, 7.0f, 9.0f, 11.0f}; /* the porch, then the room behind the glass */
    const struct Row { float z, lean, cone, penumbra; } rows[] = {
        {-1.3f, 25.0f, 60.0f, 20.0f}, {0.7f, 0.0f, 60.0f, 20.0f}, {2.7f, 0.0f, 60.0f, 20.0f},
        {5.3f, 30.0f, 45.0f, 15.0f}, {7.3f, 0.0f, 60.0f, 20.0f}};
    for (float x : xs)
        for (const Row &row : rows) {
            if (x > 5.5f && row.z > 6.0f)
                continue; /* the room ends at z 6.6 */
            if ((x == 1.0f && row.z == 0.7f) || (x == 7.0f && row.z == 2.7f))
                continue; /* a chair */
            const float lean = row.lean * 3.14159265f / 180;
            RtPoint pos = {x, 1.55f, row.z};
            RtVector dir = {0, std::cos(lean), std::sin(lean)}; /* up, leaning toward +z */
            RtColor Ic = {I[0], I[1], I[2]};
            rtSpotLight(pos, dir, Ic, row.cone, row.penumbra, 8);
        }
}

/* The lights: the sun (the file's lamp) is the key. With `fills`, a dome
 * of weak spots stands in for the sky, a grid under the roof for the
 * floor's bounce onto it, and two far spots behind the camera for the
 * sky and bounce on everything that faces it; without them (SunOnly)
 * the sun alone, for kajiya, which gathers the sky and the bounce light
 * itself. The fills' intensities are set against the kajiya render of
 * the sun-only scene at a quarter size, surface by surface, not
 * guessed. */
static void PavilionLights(bool fills)
{
    /* The sun: 11.4 degrees across, as the file has it, a disc 500 m out
     * over the pavilion. */
    RtColor sunE = {4.2f, 3.4f, 2.5f}; /* the file's 3.5 W/m^2 of (1, 0.84, 0.69): a little more, and warmer, for the look */
    SunDisc(Vec3(-0.276f, -0.821f, -0.499f), sunE, 11.4f, Vec3(-21.2f, 1.5f, 9.2f), 500.0f);
    if (!fills)
        return;
    /* The sky: 48 spots 80 m out over the scene, between 7 and 72
     * degrees up (lower ones the trees would block), set so the sunlit
     * terrace, wall and coping come out as kajiya renders them (the sky
     * map gives a surface facing up 1.3, 1.8, 2.7 W/m^2; the bounce off
     * the white roof and the stone adds the rest). */
    const Vec3 center(-10, 2, 4); /* between the pool and the house */
    RtColor sky = {1.9f, 2.0f, 2.7f};
    Dome(center, 80, 0.12f, 0.95f, 48, sky, 1);

    /* The floor's bounce onto the roof's underside: about 0.9 W/m^2 of
     * warm white on the slab 3 m up, which with the ambient is what
     * kajiya gives it. */
    RtColor bounce = {2.7f, 2.5f, 2.1f};
    CeilingGrid(bounce);

    /* The fascia, the pool's far wall, the chairs and the glass front
     * face the camera, away from the sun, and are lit by the open sky
     * over the pool and its reflection: two spots 80 m behind the camera,
     * 30 degrees to either side and 10 degrees up, each about 0.35 W/m^2
     * on a face that looks back at it, their cones holding the whole
     * scene. (What the glass front shows, the trees through two panes
     * and the sky's reflection, no fill changes.) */
    const Vec3 at(-2.0f, 3.0f, 2.0f); /* the fascia's middle */
    for (int k = 0; k < 2; k++) {
        const float side = k == 0 ? 1.0f : -1.0f;
        const Vec3 from = at + Vec3(-68.5f, 13.9f, side * 39.5f);
        const Vec3 d = normalize(at - from);
        RtPoint pos = {from.x, from.y, from.z};
        RtVector dir = {d.x, d.y, d.z};
        RtColor I = {2200.0f, 2300.0f, 2700.0f};
        rtSpotLight(pos, dir, I, 16, 6, 0);
    }
}

/* scene -------------------------------------------------------------- */

static void PavilionScene(const char *name, bool fills, int spp)
{
    RtPoint eye = {-22.17f, 1.98f, 9.44f}, lookat = {-21.21f, 1.98f, 9.15f};
    RtVector up = {0, 1, 0};
    RtColor ka = {0.09f, 0.09f, 0.1f}; /* a little, for what the dome does not reach */
    RtColor none = {0, 0, 0};

    rtBegin();
        rtFormat(1920, 1080);
        rtDisplay(name);
        rtCamera(eye, lookat, up, 31.4f);
        rtAmbient(fills ? ka : none);
        /* the file's world, baked in the renderer's lat-long convention
         * (no turn, no scale): the backdrop, and under kajiya the sky
         * light */
        rtEnvironment("env/pavilion_sky.hdr", 1.2f, 0); /* a fifth brighter: the photo is a little flat */
        rtPixelSamples(spp);
        rtSampler("halton");
        rtWorldBegin(0);
            PavilionLights(fills);

            ObjMesh *building = obj_load("models/pavilion/pavilion.mesh");
            PavilionHook hook = {building};
            ObjDraw(building, 0, PavilionMaterials, &hook);
            obj_free(building);
            ObjDrawInstances("models/pavilion/instances.txt", 0, PavilionMaterials, NULL);
        rtWorldEnd();
    rtEnd();
}

/* 64 spp is plenty with fills: every light is a delta light, so cook has
 * only the pixel and the glossy lobes to average. The sun-only entry is
 * for kajiya, at 256 spp. */
void BarcelonaPavilion(void) { PavilionScene("3_HeroScenes/BarcelonaPavilion", true, 64); }
void BarcelonaPavilionSunOnly(void) { PavilionScene("3_HeroScenes/BarcelonaPavilionSunOnly", false, 256); }
