/*
 * ECE/CS 285 – HW2 Test Scene: Nested
 * A glass sphere inside a larger one and a glass slab 0.8 m thick turned
 * 35 degrees, in front of a wall of 0.5 m stripes: entering, leaving and
 * total internal reflection. Through the slab the stripes shift 0.19 m
 * (Snell at 35 degrees, ior 1.5) and dim about 8% (two Fresnel losses).
 * With the slab's ior set to 1.0 it must disappear. 960 x 540; 256 spp,
 * halton, seed 0 under cook and kajiya.
 *
 * Output: renders/<integrator>/1_TestScenes/Nested.png
 */

#include "rt.h"
#include "shaders.h"
#include "sceneutil.h"

void Nested(void)
{
    RtPoint eye = {0, 1.2f, 7}, lookat = {0, 1.2f, 0};
    RtVector up = {0, 1, 0};
    /* One directional light, steep and from the front, so the wall is lit
     * evenly (every point of a stripe has the same radiance, which makes
     * the dimming through the slab measurable) and the shadows fall on
     * the floor, not on the stripes. */
    RtVector sunDir = {-0.1f, -1, -0.5f};
    RtColor sunE = {2.5f, 2.5f, 2.5f}; /* W/m^2 */
    RtColor ka = {0.1f, 0.1f, 0.1f};
    CheckerParams stripes = {24, 1, MATTE(0.85f, 0.85f, 0.85f), MATTE(0.12f, 0.12f, 0.14f)};
    RtMaterial floor = MATTE(0.25f, 0.25f, 0.25f);
    RtMaterial glass = GLASS;

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/Nested");
        rtPixelSamples(256);
        rtSampler("halton");
        rtCamera(eye, lookat, up, 36);
        rtAmbient(ka);
        rtMaxDepth(12); /* the nested spheres take a few bounces */
        rtWorldBegin(0);
            rtDirectionalLight(sunDir, sunE);

            /* The striped wall: 12 m wide, 6 m tall, at z = -3, facing +z;
             * 24 checks across and 1 down are 0.5 m vertical stripes. */
            rtSurface(CHECKER, &stripes, sizeof stripes);
            Quad(Vec3(-6, 6, -3), Vec3(12, 0, 0), Vec3(0, -6, 0));
            rtSurface(NULL, NULL, 0);
            rtMaterial(&floor);
            Floor(12);

            rtMaterial(&glass);
            rtTransformBegin(); rtTranslate(-1.6f, 1.2f, 0); rtSphere(1.1f); rtSphere(0.5f); rtTransformEnd();
            rtTransformBegin();
                rtTranslate(1.4f, 0, 0);
                rtRotate(35, 0, 1, 0);
                Box(Vec3(-0.8f, 0.1f, -0.4f), Vec3(0.8f, 2.3f, 0.4f));
            rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}
