/*
 * ECE/CS 285 – HW3 Test Scene: PerforatedLamp
 * Two square lamps of one radiance face the camera over a matte floor, a
 * wall between them; a cutout takes half of the right one away in
 * vertical slats. Tests that a sample of a light asks its cutout whether
 * the point is there: a thin post before each lamp casts one soft shadow
 * from the solid lamp and a fan of four from the slats, and straight out
 * from its middle the slatted lamp gives half the light.
 * 960 x 540, 256 spp, stratified, seed 0, no ambient.
 *
 * Output: renders/<integrator>/1_TestScenes/PerforatedLamp.png
 */

#include "rt.h"
#include "sceneutil.h"

/* Eight vertical slats across the lamp's u, every other one a hole, at
 * every depth: a hole is a hole for every ray and for the light sampler. */
static RtInt SLATS(RtShadeContext *c)
{
    return (int)(c->u * 8) % 2 == 0;
}

void PerforatedLamp(void)
{
    RtPoint eye = {0, 1.1f, 2.4f}, lookat = {0, 0.3f, 0.2f};
    RtVector up = {0, 1, 0};
    RtColor black = {0, 0, 0};
    RtMaterial lamp = EMITTER(10, 10, 10);
    RtMaterial grey = MATTE(0.5f, 0.5f, 0.5f);

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/PerforatedLamp");
        rtCamera(eye, lookat, up, 35);
        rtPixelSamples(256);
        rtSampler("stratified");
        rtAmbient(black);
        rtBackground(black);
        rtWorldBegin(0);
            /* The lamps: 0.5 m squares, their lower edges 0.25 m up, facing +z. */
            rtMaterial(&lamp);
            Quad(Vec3(-0.75f, 0.75f, 0), Vec3(0.5f, 0, 0), Vec3(0, -0.5f, 0));
            rtCutout(SLATS, NULL, 0);
            Quad(Vec3(0.25f, 0.75f, 0), Vec3(0.5f, 0, 0), Vec3(0, -0.5f, 0));
            rtCutout(NULL, NULL, 0);

            /* The floor, the wall between the two halves, and a post 2 cm
             * square and 0.5 m tall 0.45 m before each lamp's middle. */
            rtMaterial(&grey);
            Floor(8);
            Quad(Vec3(0, 1, -0.6f), Vec3(0, 0, 2.4f), Vec3(0, -1, 0));
            Box(Vec3(-0.51f, 0, 0.44f), Vec3(-0.49f, 0.5f, 0.46f));
            Box(Vec3(0.49f, 0, 0.44f), Vec3(0.51f, 0.5f, 0.46f));
        rtWorldEnd();
    rtEnd();
}
