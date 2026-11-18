/*
 * ECE/CS 285 – HW3 Test Scene: QuadLight
 * A 0.5 m square lamp 1.5 m over a matte floor and nothing else, no
 * ambient: tools/area_light_test.py computes its image exactly and tells
 * an unbiased area light from a biased one.
 * 960 x 540, 64 spp, stratified, seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/QuadLight.png
 */

#include "rt.h"
#include "sceneutil.h"

/* tools/area_light_test.py repeats the camera, the lamp and the floor:
 * change them there too. */
static const float LAMP_SIZE = 0.5f, LAMP_HEIGHT = 1.5f, LAMP_LE = 32.0f, FLOOR_ALBEDO = 0.5f;

void QuadLight(void)
{
    RtPoint eye = {0, 1.1f, 1.8f}, lookat = {0, 0, -0.1f};
    RtVector up = {0, 1, 0};
    RtColor black = {0, 0, 0};
    RtMaterial lamp = EMITTER(LAMP_LE, LAMP_LE, LAMP_LE);
    RtMaterial floor = MATTE(FLOOR_ALBEDO, FLOOR_ALBEDO, FLOOR_ALBEDO);
    const float h = LAMP_SIZE / 2;

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/QuadLight");
        rtCamera(eye, lookat, up, 40);
        rtPixelSamples(64);
        rtSampler("stratified");
        rtAmbient(black);
        rtBackground(black);
        rtWorldBegin(0);
            /* The lamp, facing down, above the frame: every pixel sees the floor. */
            rtMaterial(&lamp);
            Quad(Vec3(-h, LAMP_HEIGHT, h), Vec3(LAMP_SIZE, 0, 0), Vec3(0, 0, -LAMP_SIZE));
            rtMaterial(&floor);
            Floor(20);
        rtWorldEnd();
    rtEnd();
}
