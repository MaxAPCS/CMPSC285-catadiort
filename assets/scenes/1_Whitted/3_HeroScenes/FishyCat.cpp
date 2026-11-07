/*
 * ECE/CS 285 – HW2 Hero Scene: FishyCat
 * A cat with huge eyes under a fish on a string, in the grass: a coat of
 * 182,000 hair strands as ribbons, 13,000 instanced grass blades, 3,000
 * glowing flakes, and a 75 mm lens at f/0.07 that melts all but the
 * face. 15.4 million triangles. 1920 x 1080, 512 spp, halton, seed 0.
 * Model: Fishy Cat by Manu Järvinen (the Blender 2.74 splash),
 * blender.org demo files, CC0.
 *
 * Output: renders/<integrator>/3_HeroScenes/FishyCat.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

#include <cmath>
#include <cstdint>
#include <cstring>

/* Every light below, and the sky, carries the picture's color: a
 * yellow-green filter the camera looks through, (1.432, 1.588, 0.692) on
 * average (the sky has its shift from left to right across the frame),
 * times a warm white balance, (1.434, 1, 1.972). */
static const float TINT[3] = {1.432f * 1.434f, 1.588f, 0.692f * 1.972f};

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

/* SPARKLE: a flake glowing in one color of a ramp from dark red through
 * gold to teal, picked by a hash of the 2 m cell it is in (the flakes are
 * a few centimeters across and meters apart). The emission comes from the
 * shader, so the 3,000 flakes are seen, as bokeh, but are not lights. */
static void SPARKLE(RtShadeContext *c)
{
    static const float ramp[3][4] = {{0.0f, 0.32f, 0.05f, 0.05f}, {0.53f, 1.0f, 0.84f, 0.34f}, {1.0f, 0.34f, 0.50f, 0.52f}};
    const float cell[3] = {std::floor(c->P[0] * 0.5f), std::floor(c->P[1] * 0.5f), std::floor(c->P[2] * 0.5f)};
    const float x = Hash01(cell, 3);
    const int k = x < ramp[1][0] ? 0 : 1;
    const float t = (x - ramp[k][0]) / (ramp[k + 1][0] - ramp[k][0]);
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = 0;
        c->m.specular[i] = 0;
        c->m.emission[i] = TINT[i] * (ramp[k][i + 1] + t * (ramp[k + 1][i + 1] - ramp[k][i + 1]));
    }
}

/* COAT: the light that bounces from hair to hair inside the coat, which
 * cook does not trace: a glow in the hair's own color (its vertex color,
 * the albedo here), stronger the whiter the hair, since white fur is lit
 * mostly by other white fur. */
static void COAT(RtShadeContext *c)
{
    const float a = (c->m.albedo[0] + c->m.albedo[1] + c->m.albedo[2]) / 3;
    for (int i = 0; i < 3; i++)
        c->m.emission[i] = 0.048f * TINT[i] * c->m.albedo[i] * a * a * a;
}

/* WHISKER: a faint glow on top of the whiskers' matte and gloss. */
static void WHISKER(RtShadeContext *c)
{
    static const float glow[3] = {0.101f, 0.089f, 0.072f};
    for (int i = 0; i < 3; i++)
        c->m.emission[i] = TINT[i] * glow[i];
}

/* CORNEA: the glass domes over the eyes exist only for rays coming in
 * from outside (the camera's, which refract and reflect on them); a ray
 * travelling outward through a dome (a shadow ray from the iris toward
 * a light, which the glass would otherwise block, as no light refracts
 * in) passes. The eyes are centered about (+-2.3, 6.5, 0.3). */
static RtInt CORNEA(RtShadeContext *c)
{
    const Vec3 center(c->P[0] < 0 ? -2.3f : 2.3f, 6.48f, 0.32f);
    const Vec3 out = Vec3(c->P) - center;
    return dot(Vec3(c->I), out) < 0 ? 1 : 0;
}

/* NOWHERE: a light no ray hits, seen only by the light sampling. */
static RtInt NOWHERE(RtShadeContext *)
{
    return 0;
}

static void FishyCatMaterials(const char *material, void *)
{
    if (std::strcmp(material, "cat_eye_cornea") == 0)
        rtCutout(CORNEA, NULL, 0);
    if (std::strcmp(material, "cat_hair_short") == 0)
        rtSurface(COAT, NULL, 0);
    if (std::strcmp(material, "cat_hair_long") == 0)
        rtSurface(WHISKER, NULL, 0);
    if (std::strcmp(material, "env_sparkle") == 0) {
        RtMaterial m = MATTE(0, 0, 0);
        rtMaterial(&m); /* the MTL's Ke would make each flake a light */
        rtSurface(SPARKLE, NULL, 0);
    }
    if (std::strcmp(material, "env_reflector") == 0) {
        /* A bowl around the cat glowing a warm white: its biggest light,
         * hidden from the camera, from reflections and from shadows. */
        RtMaterial m = EMITTER(0.666f * TINT[0], 0.629f * TINT[1], 0.578f * TINT[2]);
        rtMaterial(&m);
        rtCutout(NOWHERE, NULL, 0);
    }
}

void FishyCat(void)
{
    /* The camera: a 75 mm lens, 13.7 degrees tall, at f/0.07, a lens
     * half a meter in radius in this big scene (the cat is 10 m tall):
     * the face, a few meters deep about the 49.8 m focus, stays nearly
     * sharp while the grass and the sparkles melt. */
    RtPoint eye = {19.40359f, -2.55386f, 48.31116f}, lookat = {18.99646f, -2.3667f, 47.41717f};
    RtVector up = {-0.01913f, 0.97682f, 0.21322f};

    /* Two suns about the cat: a warm one 11.4 degrees across from the
     * right, and a cool one 90 degrees across from the upper left (at that
     * size SunDisc delivers cos^2(45) of E, so twice the irradiance is
     * asked for). The ambient, unoccluded, is the sky's mean radiance. */
    const Vec3 cat(0.0f, 6.5f, 0.0f);
    RtColor warmE = {0.7052f * TINT[0], 0.5727f * TINT[1], 0.3755f * TINT[2]};
    RtColor coolE = {2 * 0.5106f * TINT[0], 2 * 0.8427f * TINT[1], 2 * 0.9774f * TINT[2]};
    RtColor ka = {0.110f * TINT[0], 0.107f * TINT[1], 0.088f * TINT[2]};

    rtBegin();
        rtFormat(1920, 1080);
        rtDisplay("3_HeroScenes/FishyCat");
        rtTonemap("aces", 0.1f);
        rtCamera(eye, lookat, up, 13.7f);
        rtLens(0.536f, 49.84f);
        rtPixelSamples(512);
        rtSampler("halton");
        rtMaxDepth(6);
        rtAmbient(ka);
        rtEnvironment("env/fishycat_sky.hdr", 1.0f, 0);
        rtWorldBegin(0);
            SunDisc(Vec3(-0.9432f, -0.20057f, 0.26486f), warmE, 11.4f, cat);
            SunDisc(Vec3(0.59039f, -0.72745f, -0.34966f), coolE, 90.0f, cat);
            ObjMesh *model = obj_load("models/fishycat/fishycat.mesh");
            ObjDraw(model, 0, FishyCatMaterials, NULL);
            obj_free(model);
            ObjDrawInstances("models/fishycat/instances.txt", 0, FishyCatMaterials, NULL);
        rtWorldEnd();
    rtEnd();
}
