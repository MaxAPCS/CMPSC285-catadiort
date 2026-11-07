/*
 * ECE/CS 285 – HW2 Test Scene: BallOnSlab
 * One orange ball on a beige slab under two point lights and a sun along
 * the view direction, matte materials only: shading and a shadow, no
 * highlights, no reflections. 720 x 720; 256 spp, halton, seed 0 under
 * cook and kajiya.
 *
 * Output: renders/<integrator>/1_TestScenes/BallOnSlab.png
 */

#include "rt.h"
#include "sceneutil.h"

void BallOnSlab(void)
{
    RtPoint eye = {-9.58772f, 4.26363f, 4.60779f}, lookat = {-9.00738f, 3.74035f, 3.98378f};
    RtVector up = {0.411139f, 0.849684f, -0.330155f};
    RtColor ka = {0.025f, 0.025f, 0.025f};
    RtPoint light0Pos = {-1.84647f, 0.778452f, 2.67544f};
    RtColor light0I = {60.0857f, 60.0857f, 60.0857f}; /* W/sr: 0.464 pi * (1.0f, 1.0f, 1.0f) at 6.42 m, the focal point */
    RtPoint light1Pos = {-7.78433f, 2.38677f, -5.08224f};
    RtColor light1I = {76.6313f, 76.6313f, 76.6313f}; /* W/sr: 0.464 pi * (1.0f, 1.0f, 1.0f) at 7.25 m, the focal point */
    RtVector sun0Dir = {0.580339f, -0.523277f, -0.62401f};
    RtColor sun0E = {1.4577f, 1.4577f, 1.4577f}; /* W/m^2: 0.464 pi * (1.0f, 1.0f, 1.0f) */
    RtMaterial m0 = {{0.56f, 0.35f, 0.14f}, {0.0f, 0.0f, 0.0f}, 0.269191f, 0, 0.0f, 1.5f, {0.0f, 0.0f, 0.0f}};
    RtMaterial m1 = {{0.56f, 0.5f, 0.43f}, {0.0f, 0.0f, 0.0f}, 0.269191f, 0, 0.0f, 1.5f, {0.0f, 0.0f, 0.0f}};
    static const RtPoint ps0[36] = {
        {-6.19481f, -2.18127f, 0.67082f}, {-6.19481f, -2.52771f, 0.67082f}, {3.11895f, -2.52771f, 0.67082f},
        {-6.19481f, -2.18127f, 0.67082f}, {3.11895f, -2.52771f, 0.67082f}, {3.11895f, -2.18127f, 0.67082f},
        {-6.19481f, -2.18127f, -8.92172f}, {3.11895f, -2.18127f, -8.92172f}, {3.11895f, -2.52771f, -8.92172f},
        {-6.19481f, -2.18127f, -8.92172f}, {3.11895f, -2.52771f, -8.92172f}, {-6.19481f, -2.52771f, -8.92172f},
        {-6.19481f, -2.18127f, 0.67082f}, {-6.19481f, -2.18127f, -8.92172f}, {-6.19481f, -2.52771f, -8.92172f},
        {-6.19481f, -2.18127f, 0.67082f}, {-6.19481f, -2.52771f, -8.92172f}, {-6.19481f, -2.52771f, 0.67082f},
        {3.11895f, -2.18127f, 0.67082f}, {3.11895f, -2.52771f, 0.67082f}, {3.11895f, -2.52771f, -8.92172f},
        {3.11895f, -2.18127f, 0.67082f}, {3.11895f, -2.52771f, -8.92172f}, {3.11895f, -2.18127f, -8.92172f},
        {-6.19481f, -2.18127f, 0.67082f}, {3.11895f, -2.18127f, 0.67082f}, {3.11895f, -2.18127f, -8.92172f},
        {-6.19481f, -2.18127f, 0.67082f}, {3.11895f, -2.18127f, -8.92172f}, {-6.19481f, -2.18127f, -8.92172f},
        {-6.19481f, -2.52771f, 0.67082f}, {-6.19481f, -2.52771f, -8.92172f}, {3.11895f, -2.52771f, -8.92172f},
        {-6.19481f, -2.52771f, 0.67082f}, {3.11895f, -2.52771f, -8.92172f}, {3.11895f, -2.52771f, 0.67082f},
    };

    rtBegin();
        rtFormat(720, 720);
        rtDisplay("1_TestScenes/BallOnSlab");
        rtPixelSamples(256); /* cook and kajiya; whitted renders one sample */
        rtSampler("halton");
        rtCamera(eye, lookat, up, 45.0f);
        rtAmbient(ka);
        rtMaxDepth(10);
        rtWorldBegin(0);
            rtPointLight(light0Pos, light0I);
            rtPointLight(light1Pos, light1I);
            rtDirectionalLight(sun0Dir, sun0E);

            rtMaterial(&m0);
            rtTransformBegin(); rtTranslate(-2.11537f, -0.766425f, -3.86329f); rtSphere(1.33453f); rtTransformEnd();
            rtMaterial(&m1);
            Triangles(ps0, 12);
        rtWorldEnd();
    rtEnd();
}
