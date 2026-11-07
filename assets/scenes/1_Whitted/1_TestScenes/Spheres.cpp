/*
 * ECE/CS 285 – HW2 Test Scene: Spheres
 * Three matte spheres (red, green, blue) on a grey quad under two point
 * lights and a weak directional light, no ambient. The first picture:
 * render it with --aov raydir, then --aov normal, before any lighting
 * exists, then lit. 960 x 540; 256 spp, halton, seed 0 under cook and
 * kajiya.
 *
 * Output: renders/<integrator>/1_TestScenes/Spheres.png
 */

#include "rt.h"
#include "sceneutil.h"

void Spheres(void)
{
    RtPoint eye = {0, 3, 9}, lookat = {0, 0.8f, 0};
    RtVector up = {0, 1, 0};
    RtPoint keyPos = {-4, 7, 5}, fillPos = {5, 5, 2};
    RtColor keyI = {120, 120, 120}, fillI = {60, 60, 60}; /* W/sr */
    RtVector sunDir = {0.3f, -1, -0.5f};
    RtColor sunE = {0.25f, 0.25f, 0.25f}; /* W/m^2 */

    RtMaterial grey = MATTE(0.5f, 0.5f, 0.5f);
    RtMaterial red = MATTE(0.8f, 0.15f, 0.15f);
    RtMaterial green = MATTE(0.15f, 0.7f, 0.2f);
    RtMaterial blue = MATTE(0.2f, 0.3f, 0.9f);

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/Spheres");
        rtPixelSamples(256); /* cook and kajiya render 256 spp; whitted renders one sample */
        rtSampler("halton");
        rtCamera(eye, lookat, up, 35);
        rtWorldBegin(0);
            rtPointLight(keyPos, keyI);
            rtPointLight(fillPos, fillI);
            rtDirectionalLight(sunDir, sunE);

            rtMaterial(&grey);
            Floor(16);

            rtMaterial(&red);
            rtTransformBegin(); rtTranslate(-2.4f, 1, 0.3f); rtSphere(1); rtTransformEnd();
            rtMaterial(&green);
            rtTransformBegin(); rtTranslate(0, 1, -0.9f); rtSphere(1); rtTransformEnd();
            rtMaterial(&blue);
            rtTransformBegin(); rtTranslate(2.4f, 1, 0.3f); rtSphere(1); rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}
