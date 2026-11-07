/*
 * ECE/CS 285 – HW2 Test Scene: Earth
 * HW1's earth texture on a sphere through TEXTURED, turned about y so
 * the Americas face the camera, next to a quad with uvtest.png: the
 * sphere's (u, v), the t convention (t = 0 at the top row) and rtRotate
 * turning the texture with the sphere. 960 x 540; 256 spp, halton, seed 0
 * under cook and kajiya.
 *
 * Output: renders/<integrator>/1_TestScenes/Earth.png
 */

#include "rt.h"
#include "shaders.h"
#include "sceneutil.h"

void Earth(void)
{
    RtPoint eye = {0, 0.4f, 7.5f}, lookat = {0, 0, 0};
    RtVector up = {0, 1, 0};
    RtPoint lightPos = {-3, 5, 7};
    RtColor lightI = {90, 90, 90}; /* W/sr */
    RtColor ka = {0.12f, 0.12f, 0.12f};
    RtMaterial white = MATTE(1, 1, 1); /* the textures supply the color */

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/Earth");
        rtPixelSamples(256);
        rtSampler("halton");
        rtCamera(eye, lookat, up, 35);
        rtAmbient(ka);
        rtWorldBegin(0);
            rtPointLight(lightPos, lightI);
            rtMaterial(&white);

            /* The globe. Unrotated, u = 0 (the date line) faces +z, the
             * camera, and the Americas, at u = 0.25, face +x; u is measured
             * in the sphere's own space, so rtRotate turns the map with it.
             * Turned -80 degrees about y, longitude u = 80/360 faces the
             * camera: the Americas, with the Atlantic on the right. */
            TexturedParams earth = {rtMakeTexture("earth.png", 1), -1};
            rtSurface(TEXTURED, &earth, sizeof earth);
            rtTransformBegin();
                rtTranslate(-1.9f, 0, 0);
                rtRotate(-80, 0, 1, 0);
                rtSphere(1.5f);
            rtTransformEnd();

            /* The chart on a square quad facing the camera, (s, t) = (0, 0)
             * at its upper left corner. */
            TexturedParams chart = {rtMakeTexture("uvtest.png", 1), -1};
            rtSurface(TEXTURED, &chart, sizeof chart);
            Quad(Vec3(0.6f, 1.4f, 0), Vec3(2.8f, 0, 0), Vec3(0, -2.8f, 0));
        rtWorldEnd();
    rtEnd();
}
