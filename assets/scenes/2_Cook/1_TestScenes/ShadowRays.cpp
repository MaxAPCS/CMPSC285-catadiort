/*
 * ECE/CS 285 – HW3 Test Scene: ShadowRays
 * A square lamp over a dark square card over a matte floor, all in frame,
 * at one camera ray per pixel, no ambient: what shadow rays alone do to a
 * penumbra. The plain render traces one shadow ray per pixel,
 * --light-samples 64 traces 64 stratified on the lamp, and --sampler
 * independent --light-samples 64 traces 64 at random. tools/light_samples.py
 * renders it at 1, 4 and 16 spp times --light-samples 1, 4 and 16.
 * 640 x 480, 1 spp, stratified, seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/ShadowRays.png
 */

#include "rt.h"
#include "sceneutil.h"

void ShadowRays(void)
{
    RtPoint eye = {-3.2f, 4.0f, 8.2f}, lookat = {0, 1.6f, 0};
    RtVector up = {0, 1, 0};
    RtColor black = {0, 0, 0};
    RtMaterial lamp = EMITTER(30, 30, 30);
    RtMaterial card = MATTE(0.05f, 0.05f, 0.05f);
    RtMaterial floor = MATTE(0.5f, 0.5f, 0.5f);
    const float s = 1.25f, height = 4.4f; /* the lamp's side and height; the camera sits just under it */
    const float w = 1.9f, y = 2.0f;       /* the card's side and height */

    rtBegin();
        rtFormat(640, 480);
        rtDisplay("1_TestScenes/ShadowRays");
        rtCamera(eye, lookat, up, 44);
        rtPixelSamples(1);
        rtSampler("stratified");
        rtAmbient(black);
        rtBackground(black);
        rtWorldBegin(0);
            /* The lamp, facing down. */
            rtMaterial(&lamp);
            Quad(Vec3(-s / 2, height, s / 2), Vec3(s, 0, 0), Vec3(0, 0, -s));
            /* The card, facing up, and the floor. */
            rtMaterial(&card);
            Quad(Vec3(-w / 2, y, -w / 2), Vec3(w, 0, 0), Vec3(0, 0, w));
            rtMaterial(&floor);
            Floor(100);
        rtWorldEnd();
    rtEnd();
}
