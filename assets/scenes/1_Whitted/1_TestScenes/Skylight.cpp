/*
 * ECE/CS 285 – HW2 Test Scene: Skylight
 * A mirror ball, a copper ball and a matte white ball on a light floor
 * under a sky map with the sun in it (Poly Haven's Kloofendal). Under
 * whitted the balls reflect the sky and nothing casts a shadow (the sky
 * is a backdrop, the ambient its stand-in); under cook the sky is the
 * light: the sun's sharp shadow and the sky's soft one at once; under
 * kajiya the same with the bounce light. 960 x 540, 256 spp, halton,
 * seed 0. Sky: Kloofendal 48d partly cloudy (Poly Haven, CC0).
 *
 * Output: renders/<integrator>/1_TestScenes/Skylight.png
 */

#include "rt.h"
#include "sceneutil.h"

void Skylight(void)
{
    RtPoint eye = {0, 1.6f, 6.0f}, lookat = {0, 0.7f, 0};
    RtVector up = {0, 1, 0};
    RtMaterial floor = MATTE(0.55f, 0.53f, 0.5f);
    RtMaterial mirror = MIRROR;
    RtMaterial copper = {{0, 0, 0}, {0.95f, 0.64f, 0.54f}, 0.3f, 1, 0, 1.5f, {0, 0, 0}};
    RtMaterial white = MATTE(0.8f, 0.8f, 0.8f);
    /* whitted's stand-in for the sky: its irradiance on a level surface,
     * over pi, as an ambient */
    RtColor ka = {0.5f, 0.52f, 0.58f};

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/Skylight");
        rtTonemap("filmic", 0.0f);
        rtCamera(eye, lookat, up, 32);
        rtPixelSamples(256);
        rtSampler("halton");
        rtMaxDepth(6);
        rtAmbient(ka);
        rtEnvironment("env/kloofendal_48d_partly_cloudy_puresky_2k.hdr", 1.0f, 90.0f);
        rtWorldBegin(0);
            rtMaterial(&floor);
            Floor(40);
            rtMaterial(&mirror);
            rtTransformBegin(); rtTranslate(0, 1.0f, 0); rtSphere(1.0f); rtTransformEnd();
            rtMaterial(&copper);
            rtTransformBegin(); rtTranslate(-2.3f, 0.7f, 0.8f); rtSphere(0.7f); rtTransformEnd();
            rtMaterial(&white);
            rtTransformBegin(); rtTranslate(2.3f, 0.7f, 0.8f); rtSphere(0.7f); rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}
