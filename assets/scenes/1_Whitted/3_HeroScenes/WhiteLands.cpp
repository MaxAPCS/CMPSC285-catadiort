/*
 * ECE/CS 285 – HW2 Hero Scene: WhiteLands
 * Three icebergs and four sailboats on a calm sea under a low sun, seen
 * from the air: 1.7 million triangles, most of them sculpted snow. The
 * sea is a blue Fresnel mirror over a false floor 70 cm down that stands
 * in for the file's scattering water: the bergs shadow it and the ice
 * shows through the shallows; the snow's steep faces go ice-blue in the
 * scene's shader; the mist, foam, wakes and birds are cards that show
 * through their alpha; six teal "caustic" lights sit by the boats; the
 * sky is the file's, baked; 256 spp, halton, seed 0 under cook and
 * kajiya.
 * Model: White Lands by Oksana Dobrovolska (the Blender 3.2 splash,
 * CC-BY-SA); its sky in textures/env/whitelands_sky.hdr.
 *
 * Output: renders/<integrator>/3_HeroScenes/WhiteLands.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

#include <algorithm>
#include <cmath>
#include <cstring>

/* Everything scaled by this: the file renders through Filmic. */
static const float EXPOSURE = 1.0f;

/* shaders ------------------------------------------------------------ */

/* The sea. Its surface is at y = -2.88 and the file's floor 5.4 m down.
 * The file's water is a blue mirror over a dense teal medium, absorbing
 * and scattering, which nothing here has. Here the surface is a blue
 * Fresnel mirror over a clear refracted ray, with the glow of the
 * medium's top layer as its own emission; a false floor 70 cm down
 * (SEAFLOOR) is the medium's body: the sun lights it through the
 * surface, so the bergs shadow the sea, and the ice above it shows
 * through the shallows, tinted by its depth (seaTint). Shadow rays pass
 * the surface (SEA, a cutout), so what is under it is sunlit. */
static const float SEA_LEVEL = -2.88f;
static const float SEA_FLOOR = 0.7f;                     /* the false floor's depth */
static const float SEA_SIGMA[3] = {50.0f, 2.1f, 3.8f};  /* absorbed per meter of depth, down and back up */
static const float SEA_GLOW[3] = {0.05f, 0.18f, 0.18f}; /* the medium's radiance in the sun, the file's units */

static void seaTint(const RtPoint P, float albedo[3])
{
    const float depth = SEA_LEVEL - P[1];
    if (depth <= 0)
        return;
    for (int i = 0; i < 3; i++)
        albedo[i] *= std::exp(-SEA_SIGMA[i] * depth);
}

static RtInt SEA(RtShadeContext *c) { return c->depth >= 0; }

static void WATER(RtShadeContext *c)
{
    if (c->P[1] < SEA_LEVEL - 0.05f) {
        /* the floor and the sides: all of it absorbed */
        for (int i = 0; i < 3; i++)
            c->m.albedo[i] = c->m.specular[i] = 0;
        c->m.transmission = 0;
        return;
    }
    /* a clear window over the floor, plus 0.6 of the glow as emission:
     * the light the top layer scatters toward the eye, which a floor
     * cannot give */
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = 0;
        c->m.emission[i] = 0.6f * SEA_GLOW[i] * EXPOSURE;
    }
    c->m.specular[0] = 0.04f; /* the file's blue mirror */
    c->m.specular[1] = 0.48f;
    c->m.specular[2] = 0.80f;
    c->m.transmission = 1.0f;
    c->m.ior = 1.33f;
    c->m.roughness = 0;
}

/* SEAFLOOR: the false floor, a matte teal the sun lights through the
 * surface, so the bergs shadow the sea and the shallows show the ice
 * above it; two octaves of noise (1.7 m and 0.6 m) break it up. */
static void SEAFLOOR(RtShadeContext *c)
{
    const double n = perlinNoise(c->P[0] * 0.6, c->P[2] * 0.6, 1.7) + 0.5 * perlinNoise(c->P[0] * 1.7, c->P[2] * 1.7, 3.1);
    const float v = (float)(1.0 + 0.25 * n);
    const float teal[3] = {0.18f, 0.70f, 0.98f};
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = std::min(1.0f, teal[i] * v);
        c->m.specular[i] = 0;
    }
    c->m.transmission = 0;
}

/* SNOW: the snow map (the MTL's map_Kd, loaded by ObjDraw), lightened
 * as the file's translucent mix lightens it (the file's normal map is
 * not used: its strength 10 is for subsurface snow, which smooths it
 * again; here it would be grit); the steeper a face, the more it goes
 * ice-blue, as the file's translucent cliffs do; under the sea level it
 * is tinted by its depth. A faint gloss. */
typedef struct {
    RtInt map, roughness;
} SnowParams;

static void SNOW(RtShadeContext *c)
{
    const SnowParams *p = (const SnowParams *)c->params;
    /* the file's snow reflects about half of what falls on it (its
     * subsurface takes the rest): the map, which averages 0.47, mixed
     * a little toward the file's pale blue, then 0.85 of that (the sun
     * at 0.96 of the file's and the sky at 3 put the shadow sides where
     * the subsurface puts them; the sunlit tops were over) */
    float base[3] = {0.58f, 0.64f, 0.70f};
    if (p->map >= 0) {
        RtFloat rgba[4];
        rtTexture(p->map, c->u, c->v, rgba);
        for (int i = 0; i < 3; i++)
            base[i] = 0.25f * base[i] + 0.63f * rgba[i];
    }
    const float lum = 0.2126f * base[0] + 0.7152f * base[1] + 0.0722f * base[2];
    const float ice[3] = {0.72f * lum, 0.92f * lum, 0.98f * lum};
    float steep = std::max(0.0f, std::min(1.0f, 1 - c->N[1]));
    steep = std::min(1.0f, 0.8f * steep * std::sqrt(steep));
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = std::min(0.98f, 0.85f * (base[i] + (ice[i] - base[i]) * steep));
        c->m.specular[i] = 0.04f;
    }
    c->m.roughness = 0.5f;
    if (p->roughness >= 0) {
        RtFloat rgba[4];
        rtTexture(p->roughness, c->u, c->v, rgba);
        c->m.roughness = rgba[0];
    }
    seaTint(c->P, c->m.albedo);
}

/* CARD: a billboard that shows its emission through its alpha map, soft
 * (transparent where the map is dark, straight through, ior 1), and
 * lights nothing (emission a shader sets is not a light). */
typedef struct {
    RtInt alpha;
    RtColor emission;
} CardParams;

static void CARD(RtShadeContext *c)
{
    const CardParams *p = (const CardParams *)c->params;
    float a = 1;
    if (p->alpha >= 0) {
        RtFloat rgba[4];
        rtTexture(p->alpha, c->u, c->v, rgba);
        a = rgba[0];
    }
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = 0;
        c->m.specular[i] = 0;
        c->m.emission[i] = p->emission[i] * a * EXPOSURE;
    }
    c->m.transmission = 1 - a;
    c->m.ior = 1.0f;
    c->m.roughness = 0;
}

/* The cards' cutout. Camera and bounce rays see the whole card, whose
 * shader makes it transparent where the alpha is dark; a shadow ray goes
 * straight through where the alpha is under one half, so a card shadows
 * the shape of its dense part and never its quad (a transmissive surface
 * stops shadow rays under whitted). */
static RtInt CARD_SHADOW(RtShadeContext *c)
{
    const CardParams *p = (const CardParams *)c->params;
    if (c->depth >= 0 || p->alpha < 0)
        return 1;
    RtFloat rgba[4];
    rtTexture(p->alpha, c->u, c->v, rgba);
    return rgba[0] >= 0.5f;
}

/* The cards' emission: the file's Emission color times strength. */
static const struct Card {
    const char *material;
    RtColor emission;
} CARDS[] = {
    {"Cloud", {1.0f, 1.0f, 1.0f}},
    {"Cloud.001", {0.52f, 1.0f, 1.0f}},
    {"birds1", {2.95f, 2.95f, 2.95f}},
    {"birds1.001", {1.21f, 1.21f, 1.21f}},
    {"birds1.002", {1.21f, 1.21f, 1.21f}},
    {"birds1.003", {1.21f, 1.21f, 1.21f}},
    {"ship_wake", {0.42f, 0.90f, 1.42f}},
    {"ship_wake.002", {0.52f, 1.18f, 1.25f}},
};

static void WhiteLandsMaterials(const char *material, void *user)
{
    const ObjMesh *mesh = (const ObjMesh *)user;
    const ObjMaterial *om = NULL;
    for (int i = 0; i < mesh->nmaterials; i++)
        if (std::strcmp(mesh->materials[i].name, material) == 0)
            om = &mesh->materials[i];
    if (std::strcmp(material, "water") == 0) {
        rtSurface(WATER, NULL, 0);
        rtCutout(SEA, NULL, 0);
        return;
    }
    if (std::strncmp(material, "snow_", 5) == 0) {
        /* the roughness map, loaded once per material */
        static RtInt roughness[2] = {-2, -2};
        const int k = material[5] == '1' ? 0 : 1;
        if (roughness[k] == -2)
            roughness[k] = om && om->map_Pr[0] ? rtMakeTexture(om->map_Pr, 0) : -1;
        SnowParams p = {om ? om->kdHandle : -1, roughness[k]};
        rtSurface(SNOW, &p, sizeof p);
        return;
    }
    if (std::strcmp(material, "white") == 0 && om) {
        RtMaterial m = ObjMaterialToRt(*om);
        m.albedo[0] = m.albedo[1] = m.albedo[2] = 0.8f; /* the file's sails */
        rtMaterial(&m);
        return;
    }
    for (size_t k = 0; k < sizeof CARDS / sizeof CARDS[0]; k++)
        if (std::strcmp(material, CARDS[k].material) == 0) {
            CardParams p;
            p.alpha = om ? om->dHandle : -1;
            for (int i = 0; i < 3; i++)
                p.emission[i] = CARDS[k].emission[i];
            rtSurface(CARD, &p, sizeof p);
            rtCutout(CARD_SHADOW, &p, sizeof p);
            return;
        }
}

/* lights -------------------------------------------------------------- */

/* The file's six "caustic" lights: teal spots (172 degree cones, so near
 * points) and points just above the water by the boats, P watts as
 * I = P / (4 pi) times the color. */
static const struct Caustic {
    RtPoint position;
    RtVector direction;
    RtColor I;
    int spot;
} CAUSTICS[] = {
    {{-4.97894f, -2.42541f, 12.69159f}, {-0.84492f, 0.17168f, -0.50658f}, {0.0f, 5.684f, 5.539f}, 1},
    {{-5.46426f, -2.65653f, 9.57851f}, {-0.87493f, 0.17168f, 0.4528f}, {7.031f, 23.455f, 27.780f}, 1},
    {{-4.05964f, -2.6415f, 13.04741f}, {-0.60854f, 0.17168f, 0.77473f}, {3.359f, 10.745f, 11.056f}, 1},
    {{-2.46109f, -2.15519f, 12.61441f}, {0, -1, 0}, {1.479f, 5.570f, 7.106f}, 0},
    {{-3.97138f, -2.41951f, 12.58596f}, {0, -1, 0}, {0.0f, 5.684f, 5.539f}, 0},
    {{-5.25894f, -2.70632f, 12.7257f}, {-0.21289f, -0.15143f, 0.96527f}, {0.109f, 1.907f, 2.122f}, 1},
};

/* scene -------------------------------------------------------------- */

void WhiteLands(void)
{
    RtPoint eye = {1.513f, -0.992f, 13.285f}, lookat = {0.578f, -1.317f, 13.148f};
    RtVector up = {-0.322f, 0.946f, -0.047f};
    /* The sun: the file's (20, 18.3, 10.3) at 0.96 (its strength is 1.2;
     * the snow has no subsurface to soften its light), 8 degrees above
     * the horizon, as a 3 degree disc so the sculpted snow's self-shadows
     * blur as the file's subsurface blurs them. */
    RtVector sunDir = {-0.115f, -0.143f, -0.983f};
    RtColor sunE = {19.2f * EXPOSURE, 17.6f * EXPOSURE, 9.9f * EXPOSURE};
    /* The sky's light on the shadow sides, which the baked sky cannot
     * give whitted, and the snow's bounce: a cool ambient. */
    RtColor ka = {0.05f * EXPOSURE, 0.085f * EXPOSURE, 0.14f * EXPOSURE};
    /* The caustic lights at a twelfth of the file's power: the file
     * shines each through a Voronoi mask whose mean is about that. */
    const float CAUSTIC = 0.08f;

    rtBegin();
        rtFormat(1920, 1080);
        rtDisplay("3_HeroScenes/WhiteLands");
        rtPixelSamples(256); /* cook and kajiya; whitted renders one sample */
        rtSampler("halton");
        rtTonemap("filmic", 1.3f);
        rtCamera(eye, lookat, up, 22.9f);
        rtAmbient(ka);
        rtEnvironment("env/whitelands_sky.hdr", 3.0f * EXPOSURE, 0); /* the baked sky at 3: the shadow sides' fill */
        rtWorldBegin(0);
            SunDisc(Vec3(sunDir), sunE, 3.0f, Vec3(0.6f, -1.3f, 13.1f), 2000.0f); /* 2 km out */
            for (size_t i = 0; i < sizeof CAUSTICS / sizeof CAUSTICS[0]; i++) {
                RtPoint pos = {CAUSTICS[i].position[0], CAUSTICS[i].position[1], CAUSTICS[i].position[2]};
                RtVector dir = {CAUSTICS[i].direction[0], CAUSTICS[i].direction[1], CAUSTICS[i].direction[2]};
                RtColor I = {CAUSTICS[i].I[0] * CAUSTIC * EXPOSURE, CAUSTICS[i].I[1] * CAUSTIC * EXPOSURE,
                             CAUSTICS[i].I[2] * CAUSTIC * EXPOSURE};
                if (CAUSTICS[i].spot)
                    rtSpotLight(pos, dir, I, 86.0f, 30.0f, 0.0f);
                else
                    rtPointLight(pos, I);
            }

            ObjMesh *world = obj_load("models/whitelands/whitelands.mesh");
            ObjDraw(world, 0, WhiteLandsMaterials, world);
            obj_free(world);
            RtMaterial floor = MATTE(1, 1, 1);
            rtMaterial(&floor);
            rtSurface(SEAFLOOR, NULL, 0);
            Quad(Vec3(-26, SEA_LEVEL - SEA_FLOOR, -2), Vec3(34, 0, 0), Vec3(0, 0, 36)); /* under the file's water box */
        rtWorldEnd();
    rtEnd();
}
