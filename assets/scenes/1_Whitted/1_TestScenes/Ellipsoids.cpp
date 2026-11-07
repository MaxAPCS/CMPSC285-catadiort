/*
 * ECE/CS 285 – HW2 Test Scene: Ellipsoids
 * The Spheres scene with the balls stretched into upright ellipsoids
 * (rtSphere under a non-uniform rtScale), the red one leaning 25 degrees
 * left, the blue one 25 right: the sphere test under a CTM and normals
 * through the inverse transpose. 960 x 540; 256 spp, halton, seed 0 under
 * cook and kajiya.
 *
 * Output: renders/<integrator>/1_TestScenes/Ellipsoids.png
 */

#include "rt.h"
#include "sceneutil.h"

void Ellipsoids(void)
{
    RtPoint eye = {0, 3, 9}, lookat = {0, 0.8f, 0};
    RtVector up = {0, 1, 0};
    /* The lights sit higher than in Spheres so that no ellipsoid shadows
     * its neighbor: with no ambient such a shadow is pitch black. */
    RtPoint keyPos = {-4, 8, 5}, fillPos = {5, 8, 3};
    RtColor keyI = {140, 140, 140}, fillI = {110, 110, 110}; /* W/sr */
    RtVector sunDir = {0.3f, -1, -0.5f};
    RtColor sunE = {0.25f, 0.25f, 0.25f}; /* W/m^2 */

    RtMaterial grey = MATTE(0.5f, 0.5f, 0.5f);
    RtMaterial red = MATTE(0.8f, 0.15f, 0.15f);
    RtMaterial green = MATTE(0.15f, 0.7f, 0.2f);
    RtMaterial blue = MATTE(0.2f, 0.3f, 0.9f);

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/Ellipsoids");
        rtPixelSamples(256);
        rtSampler("halton");
        rtCamera(eye, lookat, up, 35);
        rtWorldBegin(0);
            rtPointLight(keyPos, keyI);
            rtPointLight(fillPos, fillI);
            rtDirectionalLight(sunDir, sunE);

            rtMaterial(&grey);
            Floor(16);

            /* The last call before rtSphere applies to the sphere first:
             * scale, then rotate, then translate. A tilted egg's lowest
             * point is its vertical half-extent below the center,
             * sqrt((0.8 sin 25)^2 + (1.4 cos 25)^2) = 1.313, so that is its
             * center's height; the upright one sits at 1.4. */
            rtMaterial(&red);
            rtTransformBegin();
                rtTranslate(-2.4f, 1.313f, 0.3f);
                rtRotate(25, 0, 0, 1); /* the top leans toward -x */
                rtScale(0.8f, 1.4f, 0.8f);
                rtSphere(1);
            rtTransformEnd();
            rtMaterial(&green);
            rtTransformBegin();
                rtTranslate(0, 1.4f, -0.9f);
                rtScale(0.8f, 1.4f, 0.8f);
                rtSphere(1);
            rtTransformEnd();
            rtMaterial(&blue);
            rtTransformBegin();
                rtTranslate(2.4f, 1.313f, 0.3f);
                rtRotate(-25, 0, 0, 1); /* the top leans toward +x */
                rtScale(0.8f, 1.4f, 0.8f);
                rtSphere(1);
            rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}
