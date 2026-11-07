/*
 * ECE/CS 285 – HW2 Test Scene: WhittedSpheres
 * The 1980 homage: a CHECKER floor, a glass sphere (GLASS with ior 1.8)
 * and a mirror sphere (MIRROR), one point light, ambient 0.05, a light
 * blue sky through rtBackground. Reflection, refraction with a Fresnel
 * rim, and secondary rays that leave the scene returning the background.
 * 960 x 540; 256 spp, halton, seed 0 under cook and kajiya.
 *
 * Output: renders/<integrator>/1_TestScenes/WhittedSpheres.png
 */

#include "rt.h"
#include "shaders.h"
#include "sceneutil.h"

void WhittedSpheres(void)
{
    RtPoint eye = {0, 2.4f, 7.5f}, lookat = {0, 1.0f, 0};
    RtVector up = {0, 1, 0};
    RtPoint lightPos = {-3, 8, 5};
    RtColor lightI = {160, 160, 160}; /* W/sr */
    RtColor ka = {0.05f, 0.05f, 0.05f};
    CheckerParams checks = {12, 12, MATTE(0.9f, 0.85f, 0.3f), MATTE(0.75f, 0.12f, 0.08f)};
    RtMaterial glass = GLASS, mirror = MIRROR;
    glass.ior = 1.8f; /* denser than GLASS's 1.5: twice the head-on Fresnel reflection */

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/WhittedSpheres");
        rtPixelSamples(256);
        rtSampler("halton");
        rtCamera(eye, lookat, up, 38);
        rtAmbient(ka);
        rtBackground(Color8(67, 178, 221)); /* a light blue sky, given as sRGB bytes */
        rtWorldBegin(0);
            rtPointLight(lightPos, lightI);

            rtSurface(CHECKER, &checks, sizeof checks);
            Floor(12); /* 12 m, so the checks are 1 m */
            rtSurface(NULL, NULL, 0);

            rtMaterial(&glass);
            rtTransformBegin(); rtTranslate(-1.1f, 1.7f, 1.2f); rtSphere(1); rtTransformEnd();
            rtMaterial(&mirror);
            rtTransformBegin(); rtTranslate(1.5f, 1.0f, -1.0f); rtSphere(1); rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}
