/*
 * ECE/CS 285 – HW2 Hero Scene: AtticCloseup
 * A desk under the eaves, straight down: a book, an oil lamp of glass
 * and tarnished metal, coins, a key, letters and cobwebs, lit by one
 * shaft of sun broken by a leafy gobo, with the attic's HDRI as the
 * fill (ambient and occlusion under cook). 138,000 triangles.
 * 1920 x 1080, 1024 spp, halton, seed 0, max depth 6, filmic.
 * Model: Attic Close-up by Gleb Alexandrov, blender.org demo files,
 * CC-BY; its world in textures/env/attic_env.hdr.
 *
 * Output: renders/<integrator>/3_HeroScenes/AtticCloseup.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

#include <cstring>

/* shaders ------------------------------------------------------------ */

/* GOBO: the card between the sun and the desk, cut into a leafy pattern
 * by noise, so the sun comes through dappled (the file does it with a
 * shadow image the download lacks). Solid where the noise is above the
 * threshold; the card's own look does not matter, it is out of frame. */
typedef struct {
    RtFloat scale, threshold;
} GoboParams;

static RtInt GOBO(RtShadeContext *c)
{
    const GoboParams *p = (const GoboParams *)c->params;
    const double n = Fbm(c->P[0] * p->scale, c->P[1] * p->scale, c->P[2] * p->scale, 4);
    return n > p->threshold ? 1 : 0;
}

static void AtticMaterials(const char *material, void *)
{
    if (std::strcmp(material, "curtain_01") == 0) {
        GoboParams g = {9.0f, 0.22f}; /* mostly open: leaf-sized blockers where the noise peaks */
        rtCutout(GOBO, &g, sizeof g);
    }
}

/* scene -------------------------------------------------------------- */

void AtticCloseup(void)
{
    /* The file's camera, 1 m above the desk looking down a little
     * forward: a 35 mm lens on a 32 mm sensor, 28.8 degrees tall. */
    RtPoint eye = {0.1833f, 1.85715f, 0.27588f}, lookat = {0.18091f, 0.92357f, -0.08249f};
    RtVector up = {-0.00451f, 0.35838f, -0.93356f};

    /* The sun: strength 100 times (1, 0.795, 0.58), 20 degrees up, from
     * the right and behind the camera, the hard shard across the book;
     * two 1 W points over the desk (hardly anything); and the world, an
     * HDRI whose own sun (measured on the map: irradiance (0.92, 1.03,
     * 1.21) within 0.09 sr about (-0.43, 0.85, -0.31)) lights the whole
     * desk softly from high on the left: here an emitting disc 20 degrees
     * across, and the rest of the sky as the ambient (radiance 0.1, so
     * pi x 0.1), with occlusion within 0.2 m. */
    const float EXPOSURE = 0.5f;
    RtVector sunDir = {-0.71292f, -0.33888f, 0.61392f};
    RtColor sunE = {100.0f * EXPOSURE, 79.53f * EXPOSURE, 58.15f * EXPOSURE};
    RtColor skySunE = {1.05f * EXPOSURE, 1.03f * EXPOSURE, 1.0f * EXPOSURE}; /* the map's (0.92, 1.03, 1.21), warmed as the original's grade is */
    RtPoint p1 = {0.48645f, 0.91976f, -0.14609f}, p2 = {0.01327f, 0.99413f, 0.03211f};
    RtColor pointI = {0.08f * EXPOSURE, 0.08f * EXPOSURE, 0.08f * EXPOSURE};
    RtColor ka = {0.38f * EXPOSURE, 0.33f * EXPOSURE, 0.27f * EXPOSURE};

    rtBegin();
        rtFormat(1920, 1080);
        rtDisplay("3_HeroScenes/AtticCloseup");
        rtTonemap("filmic", 1.0f);
        rtCamera(eye, lookat, up, 28.842f);
        rtPixelSamples(1024);
        rtSampler("halton");
        rtMaxDepth(6);
        rtAmbient(ka);
        rtAmbientOcclusion(0.2f);
        rtEnvironment("env/attic_env.hdr", EXPOSURE, 0);
        rtWorldBegin(0);
            rtDirectionalLight(sunDir, sunE);
            SunDisc(Vec3(0.4319f, -0.8484f, 0.3062f), skySunE, 20.0f, Vec3(0.2f, 0.9f, 0.0f));
            rtPointLight(p1, pointI);
            rtPointLight(p2, pointI);
            ObjMesh *desk = obj_load("models/attic/attic.mesh");
            ObjDraw(desk, 0, AtticMaterials, NULL);
            obj_free(desk);
        rtWorldEnd();
    rtEnd();
}
