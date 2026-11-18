/*
 * ECE/CS 285 – HW3 Test Scene: ScatteringPebbles
 * The thin lens at its limit: an f/2 lens focused 1.5 m out over forty
 * thousand pebbles (three shapes, placed through instances.txt) on a dune,
 * so the near and far pebbles dissolve into bokeh, under a low sun 10
 * degrees across (every shadow soft), a blue fill sun and a red spot;
 * 10.6 million triangles. 960 x 540, 256 spp, halton, seed 0.
 * Model: Scattering Pebbles, Blender Studio (blender.org demo files), CC0.
 *
 * Output: renders/<integrator>/1_TestScenes/ScatteringPebbles.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

/* lights -------------------------------------------------------------- */

/* The suns are emitting discs (SunDisc, sceneutil.h), so their shadows
 * are soft under cook and whitted gets the same irradiance from the
 * stand-in point light. */

/* Everything scaled by this: the file renders through Filmic, this
 * renderer clamps linear radiance. */
static const float EXPOSURE = 1.0f;

static void PebbleLights(void)
{
    const Vec3 about(-2.0f, 0, 2.0f); /* the pebbles in view */

    /* The key: Blender's sun of strength 6, 10 degrees across, warm, 26
     * degrees above the horizon. */
    RtColor keyE = {6.0f * EXPOSURE, 4.2024f * EXPOSURE, 2.9662f * EXPOSURE};
    SunDisc(Vec3(-0.685f, -0.44256f, 0.57872f), keyE, 10.0f, about);

    /* The fill: a blue sun of strength 1, 25 degrees across, from the
     * other side. */
    RtColor fillE = {0.2904f * EXPOSURE, 0.396f * EXPOSURE, 1.0f * EXPOSURE};
    SunDisc(Vec3(0.78449f, -0.5591f, 0.26829f), fillE, 25.0f, about);

    /* The red disc lamp, 2 m across and 2 m up, 11.78 W, as a spot: as a
     * disc it would shade the foreground from the key (Cycles' lamps block
     * nothing). A Lambertian emitter of P watts has P / pi W/sr on its
     * axis, here over a 90 degree cone fading from 40. */
    RtPoint lampPos = {1.51217f, 1.93444f, -0.13839f};
    RtVector lampDir = {-0.65303f, -0.75715f, 0.01673f};
    const float I = 11.781f / 3.14159265f;
    RtColor lampI = {I * 1.0f * EXPOSURE, I * 0.3288f * EXPOSURE, I * 0.1721f * EXPOSURE};
    rtSpotLight(lampPos, lampDir, lampI, 90.0f, 50.0f, 0.0f);
}

/* scene -------------------------------------------------------------- */

void ScatteringPebbles(void)
{
    /* The file's camera: a 50 mm lens on a 36 mm sensor, 22.9 degrees
     * tall at 16:9; f/2 is a 12.5 mm lens radius, focused 1.5 m out. */
    RtPoint eye = {-2.92916f, 0.45291f, 3.66561f}, lookat = {-2.43706f, 0.1813f, 2.83853f};
    RtVector up = {0.1389f, 0.96241f, -0.23341f};
    /* The world: a near-black blue, the backdrop. The ambient term (cook,
     * with occlusion within 0.3 m) stands in for the sun bounced off the
     * ground, which lights the pebbles' shadow sides in the original:
     * about the ground's albedo times the sun's irradiance on it. kajiya
     * ignores it and gets the bounce itself. */
    RtColor world = {0.0085f * EXPOSURE, 0.0127f * EXPOSURE, 0.0183f * EXPOSURE};
    RtColor bounce = {0.30f * EXPOSURE, 0.20f * EXPOSURE, 0.15f * EXPOSURE};

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/ScatteringPebbles");
        rtCamera(eye, lookat, up, 22.895f);
        rtLens(0.0125f, 1.5f);
        rtPixelSamples(256);
        rtSampler("halton");
        rtMaxDepth(4);
        rtBackground(world);
        rtAmbient(bounce);
        rtAmbientOcclusion(0.3f);
        rtWorldBegin(0);
            PebbleLights();
            ObjMesh *ground = obj_load("models/pebbles/pebbles.mesh");
            ObjDraw(ground);
            obj_free(ground);
            ObjDrawInstances("models/pebbles/instances.txt");
        rtWorldEnd();
    rtEnd();
}
