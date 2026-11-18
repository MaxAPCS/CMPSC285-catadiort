/*
 * ECE/CS 285 – HW3 Hero Scene: MagicalClock
 * A brass clock walking on spider legs across a station platform, a
 * glass face over a dark dial, smoke and sparks at its feet, papers in
 * the air and the station painted on a card behind; 1.6 million
 * triangles. Lit as the file is: a soft box key and a blue rim (both
 * emitters), two cyan spots, a point light under the base, and ambient
 * with occlusion for the cabin sky; a lens of radius 4 cm focused on the
 * dial 24.4 m away; the gallery picture's grade as aces at +0.5 stops, a
 * cool tint and a vignette (rtColorGrade). 1920 x 1080, 1024 spp,
 * halton, seed 0, max depth 5.
 * Model: Magical Clock by Gleb Alexandrov (blender.org gallery, CC-BY),
 * with the HDRI cabin by Greg Zaal (CC-BY), its sky in
 * textures/env/magicalclock_sky.hdr.
 *
 * Output: renders/<integrator>/3_HeroScenes/MagicalClock.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"
#include "shaders.h"

#include <cmath>
#include <cstdint>
#include <cstring>

/* shaders ------------------------------------------------------------ */

/* GLOW_MAP: a card that shows its picture at `scale` times the map
 * whatever the lighting: no albedo, no specular, the picture as the
 * material's emission. It is not a light source: the light list is built
 * at rtWorldEnd from the materials bound to the draw calls, before any
 * shader runs, so no shadow ray or light sample reaches an emission a
 * shader sets. Under whitted and cook such a card lights nothing; under
 * kajiya only the paths that happen to hit it pick up its glow (rt.h).
 * Bound as the material's emission instead, each card would be an area
 * light, sampled at every hit (Whitted Illumination handout). */
typedef struct {
    RtInt map;
    RtColor scale;
} GlowMapParams;

static void GLOW_MAP(RtShadeContext *c)
{
    const GlowMapParams *p = (const GlowMapParams *)c->params;
    RtFloat rgba[4];
    rtTexture(p->map, c->u, c->v, rgba);
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = 0;
        c->m.specular[i] = 0;
        c->m.emission[i] = p->scale[i] * rgba[i];
    }
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

/* COVERAGE: a cutout for a semi-transparent card. A cutout answers yes or
 * no, so the card is solid with probability `coverage` times the mask
 * (white where the smoke is), decided by a hash of the hit point and ray
 * direction; the samples of a pixel, and its shadow rays, average out. */
typedef struct {
    RtInt mask;
    RtFloat coverage;
} CoverageParams;

static RtInt COVERAGE(RtShadeContext *c)
{
    const CoverageParams *p = (const CoverageParams *)c->params;
    RtFloat rgba[4];
    rtTexture(p->mask, c->u, c->v, rgba);
    const float key[6] = {c->P[0], c->P[1], c->P[2], c->I[0], c->I[1], c->I[2]};
    return Hash01(key, 6) < p->coverage * rgba[0];
}

/* TRIPLANAR: the albedo from a map projected along the world's three axes,
 * each projection weighted by |N|^4 along it (normalized), the map
 * repeating every `tile` meters, times `gain`: Blender's box projection
 * of the brass, which no per-vertex OBJ texture coordinate can carry. */
typedef struct {
    RtInt map;
    RtFloat tile;
    RtFloat gain;
} TriplanarParams;

static void TRIPLANAR(RtShadeContext *c)
{
    const TriplanarParams *p = (const TriplanarParams *)c->params;
    float w[3], sum = 0;
    for (int i = 0; i < 3; i++) {
        const float n2 = c->N[i] * c->N[i];
        w[i] = n2 * n2;
        sum += w[i];
    }
    /* the two axes each projection reads: along x it sees (z, y), along
     * y (x, z), along z (x, y) */
    const int axes[3][2] = {{2, 1}, {0, 2}, {0, 1}};
    float albedo[3] = {0, 0, 0};
    for (int k = 0; k < 3; k++) {
        if (w[k] <= 0) {
            continue;
        }
        RtFloat rgba[4];
        rtTexture(p->map, c->P[axes[k][0]] / p->tile, -c->P[axes[k][1]] / p->tile, rgba);
        for (int i = 0; i < 3; i++) {
            albedo[i] += w[k] / sum * rgba[i];
        }
    }
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = p->gain * albedo[i];
    }
}

/* the hook ------------------------------------------------------------ */

/* What the hook needs: the mesh (for the maps ObjDraw loaded) and the
 * emission maps, which obj.h does not read (the MTL's map_Ke). */
typedef struct {
    ObjMesh *mesh;
    RtInt station, smoke, sparks;
} ClockHook;

static const ObjMaterial *FindMaterial(const ObjMesh *mesh, const char *name)
{
    for (int i = 0; i < mesh->nmaterials; i++) {
        if (std::strcmp(mesh->materials[i].name, name) == 0) {
            return &mesh->materials[i];
        }
    }
    return NULL;
}

/* The material bound now, less its emission: these cards glow through
 * GLOW_MAP instead of being area lights. */
static void BindWithoutEmission(const ObjMaterial *om)
{
    RtMaterial m = ObjMaterialToRt(*om);
    m.emission[0] = m.emission[1] = m.emission[2] = 0;
    rtMaterial(&m);
}

static void ClockMaterials(const char *material, void *user)
{
    const ClockHook *hook = (const ClockHook *)user;
    const ObjMaterial *om = FindMaterial(hook->mesh, material);
    if (!om) {
        return;
    }

    /* The brass, box-projected: the map repeats every 4 m, about once
     * across the clock's body, as the original's Generated coordinates
     * have it. */
    if (std::strcmp(material, "metal_01") == 0 || std::strcmp(material, "metal_02") == 0) {
        TriplanarParams tp = {om->kdHandle, 4.0f, 1.0f};
        rtSurface(TRIPLANAR, &tp, sizeof tp);
        return;
    }
    /* The dial plates: the same projection of the greyed texture, at
     * 0.56 (the original's Fresnel takes 0.44 of the light for the gloss,
     * which cook adds on top). */
    if (std::strcmp(material, "white_mat_01") == 0) {
        TriplanarParams tp = {om->kdHandle, 4.0f, 0.56f};
        rtSurface(TRIPLANAR, &tp, sizeof tp);
        return;
    }
    /* The glass face is clear in the file; the exporter wrote its
     * roughness as Ns 6, which the OBJ loader reads as 0.71, and under
     * cook that would frost it (glossy transmission, 2026-10-30). */
    if (std::strcmp(material, "glass_01") == 0) {
        RtMaterial m = ObjMaterialToRt(*om);
        m.roughness = 0;
        rtMaterial(&m);
        return;
    }
    /* The station: a card 100 m wide and 140 m behind the clock, its
     * emission the painting at 3.5, 3.2, 2.8 times the map. The file's
     * compositor lifts the background behind the clock with masked light
     * nodes; the card's own brightness stands in for them (fitted to the
     * file's graded render, 2026-11-01). */
    if (std::strcmp(material, "background_01") == 0) {
        BindWithoutEmission(om);
        GlowMapParams ep = {hook->station, {3.53f, 3.18f, 2.75f}};
        rtSurface(GLOW_MAP, &ep, sizeof ep);
        return;
    }
    /* The smoke: six cards of emission 5 times the cloud texture, of which
     * the original's mix keeps 7.1%, so 0.355 before the file's compositor
     * turns it teal: 0.19, 0.30, 0.37 (fitted, 2026-11-01); and at most
     * 78.7% opaque where the texture's alpha says smoke (the mask). */
    if (std::strcmp(material, "smoke_01_1") == 0) {
        BindWithoutEmission(om);
        GlowMapParams ep = {hook->smoke, {0.190f, 0.301f, 0.372f}};
        rtSurface(GLOW_MAP, &ep, sizeof ep);
        CoverageParams cp = {om->dHandle, 0.787f};
        rtCutout(COVERAGE, &cp, sizeof cp);
        return;
    }
    /* The sparks: emission 3 times their texture; ObjDraw's ALPHA_CUTOUT
     * (map_d) keeps only the specks. */
    if (std::strcmp(material, "sparks_01_1") == 0) {
        BindWithoutEmission(om);
        GlowMapParams ep = {hook->sparks, {3.0f, 3.0f, 3.0f}};
        rtSurface(GLOW_MAP, &ep, sizeof ep);
        return;
    }
}

/* scene -------------------------------------------------------------- */

void MagicalClock(void)
{
    /* Blender's camera: a 50 mm lens on a 36 mm sensor fitted across, so
     * 39.6 degrees wide and, at 1920 x 1080, 22.89 degrees tall. f/0.625
     * is a lens radius of 0.05 / (2 x 0.625) = 4 cm, focused on the
     * "focus_01" empty at the dial, 24.36 m along the view. */
    RtPoint eye = {20.2208f, 8.7983f, -14.9204f}, lookat = {19.4872f, 8.9628f, -14.2610f};
    RtVector up = {0.16834f, 0.984f, -0.05829f};

    /* The sky's light, which cook cannot gather, as an ambient term: the
     * baked sky's cosine-weighted mean over the upper hemisphere, with
     * occlusion within 3 m so the clock's underside stays dark. */
    RtColor ka = {0.53f, 0.39f, 0.32f};

    /* The spots and the point light: Cycles' P watts are I = P / (4 pi)
     * W/sr times the color. A spot's size is its full cone; its blend
     * fades the edge over blend x (1 - cos(half angle)) in cosine: all of
     * the narrow 6 degrees (blend 1), the outer 1.44 of the wide 18.39. */
    RtVector down = {0, -1, 0};
    RtPoint narrowPos = {5.5398f, 33.1539f, -4.1844f};
    RtColor narrowI = {1709.8f, 47169.4f, 39350.9f};   /* light_spot_02, 592,748 W */
    RtPoint widePos = {13.1304f, 24.9434f, -4.0413f};
    RtColor wideI = {309.5f, 2132.4f, 2107.7f};        /* light_spot_01, 26,796 W */
    RtPoint glowPos = {0.1769f, 8.9979f, -0.3785f};    /* light_point_01, under the base */
    RtColor glowI = {2.979f, 71.85f, 95.49f};          /* 1,200 W */

    /* The soft boxes (Blender's area lights) as emitting quads. Cycles
     * gives P watts over area A the radiance P / (pi A); these are set
     * at P / (4 A), 0.79 of that, calibrated by eye: the key 36.05 x
     * (0.962, 0.988, 1), the rim 46.44 x (0.0355, 0.1255, 0.2165). Cycles
     * hides lamps from the camera; here the key loses a 7% strip at the
     * frame's corner and the rim is 2 m square, not 12, at equal power. */
    RtMaterial key = EMITTER(37.29f, 38.30f, 38.76f);
    RtMaterial rim = EMITTER(59.73f, 211.1f, 364.2f);

    rtBegin();
        rtFormat(1920, 1080);
        rtDisplay("3_HeroScenes/MagicalClock");
        rtCamera(eye, lookat, up, 22.89f);
        rtLens(0.04f, 24.36f);
        rtPixelSamples(1024);
        rtSampler("halton");
        /* The gallery picture's look is a compositor grade over the Cycles
         * render: a cyan cast, curves and a vignette. The lights above are
         * the file's; the grade goes on the PNG (fitted to the file's own
         * graded render at a quarter size, 2026-11-01). */
        rtTonemap("aces", 0.5f);
        RtColor tint = {0.70f, 1.0f, 0.98f};
        rtColorGrade(tint, 0.68f);
        /* Depth 5: the dial sits behind enough glass that at 4 the face
         * turns into a black mirror; deeper changes nothing visible and
         * costs about 1.4 times as much per level (each glass hit splits
         * in two). */
        rtMaxDepth(5);
        rtAmbient(ka);
        rtAmbientOcclusion(3.0f);
        /* The cabin sky, baked in the renderer's own lat-long convention
         * with the file's turn, grade and strength (no turn, no scale
         * here). Seen in the brass and the glass; the station card hides
         * it from the camera. */
        rtEnvironment("env/magicalclock_sky.hdr", 1, 0);
        rtWorldBegin(0);
            rtSpotLight(narrowPos, down, narrowI, 6.0f, 6.0f, 0);
            rtSpotLight(widePos, down, wideI, 18.39f, 1.44f, 0);
            rtPointLight(glowPos, glowI);

            rtMaterial(&key);
            Quad(Vec3(-9.7974f, 27.9473f, 48.5882f), Vec3(28.6742f, 1.6773f, -4.3423f),
                 Vec3(-3.4510f, 28.7058f, -11.7003f));
            rtMaterial(&rim);
            Quad(Vec3(-11.9815f, 18.5324f, -4.4458f), Vec3(0.6993f, 1.8721f, -0.0799f),
                 Vec3(0.9006f, -0.4106f, -1.7380f));

            ObjMesh *clock = obj_load("models/magicalclock/magicalclock.mesh");
            ClockHook hook = {clock,
                              rtMakeTexture("models/magicalclock/textures/background_01_1_background_01.jpg", 1),
                              rtMakeTexture("models/magicalclock/textures/smoke_01___smoke_01_1.jpg", 1),
                              rtMakeTexture("models/magicalclock/textures/sparks_01_1.png", 1)};
            ObjDraw(clock, 0, ClockMaterials, &hook);
            obj_free(clock);
        rtWorldEnd();
    rtEnd();
}
