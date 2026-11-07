/*
 * ECE/CS 285 – HW2 Test Scene: Cutout
 * A sphere with CHECKER_CUTOUT and a quad with ALPHA_CUTOUT on a leaf
 * texture with an alpha mask, over a floor: the cutout shader on camera
 * rays and on shadow rays, the sphere's far intersection through a hole,
 * and the image's alpha read as the mask. 960 x 540; 256 spp, halton,
 * seed 0 under cook and kajiya.
 *
 * Output: renders/<integrator>/1_TestScenes/Cutout.png
 */

#include "rt.h"
#include "shaders.h"
#include "sceneutil.h"

void Cutout(void)
{
    RtPoint eye = {0, 2.6f, 7.5f}, lookat = {0, 1.0f, 0};
    RtVector up = {0, 1, 0};
    RtPoint lightPos = {-3, 7, 4};
    RtColor lightI = {130, 130, 130}; /* W/sr */
    RtVector sunDir = {0.5f, -1, -0.3f};
    RtColor sunE = {0.3f, 0.3f, 0.3f};  /* W/m^2 */
    RtColor ka = {0.05f, 0.05f, 0.05f};
    RtMaterial grey = MATTE(0.55f, 0.55f, 0.55f);
    RtMaterial orange = MATTE(0.9f, 0.45f, 0.1f);
    RtMaterial white = MATTE(1, 1, 1); /* the leaf texture supplies the color */
    CheckerParams holes = {12, 6}; /* the materials are not read by CHECKER_CUTOUT */

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/Cutout");
        rtPixelSamples(256);
        rtSampler("halton");
        rtCamera(eye, lookat, up, 38);
        rtAmbient(ka);
        rtWorldBegin(0);
            rtPointLight(lightPos, lightI);
            rtDirectionalLight(sunDir, sunE);

            rtMaterial(&grey);
            Floor(12);

            /* The sphere: solid on the a checks, holes on the b checks, so
             * the floor and its own inside show through. */
            rtMaterial(&orange);
            rtCutout(CHECKER_CUTOUT, &holes, sizeof holes);
            rtTransformBegin(); rtTranslate(-1.7f, 1.2f, 0); rtSphere(1.1f); rtTransformEnd();
            rtCutout(NULL, NULL, 0);

            /* The leaf: its PNG's alpha is the mask, read by ALPHA_CUTOUT
             * (alphaTex -1 means the albedo image's own alpha). */
            TexturedParams leafTex = {rtMakeTexture("leaf.png", 1), -1};
            CutoutParams leafCut = {leafTex, 0.5f};
            rtMaterial(&white);
            rtSurface(TEXTURED, &leafTex, sizeof leafTex);
            rtCutout(ALPHA_CUTOUT, &leafCut, sizeof leafCut);
            rtTransformBegin();
                rtTranslate(1.6f, 1.3f, 0.2f);
                rtRotate(-20, 0, 1, 0);
                Quad(Vec3(-1.3f, 1.3f, 0), Vec3(2.6f, 0, 0), Vec3(0, -2.6f, 0));
            rtTransformEnd();
            rtSurface(NULL, NULL, 0);
            rtCutout(NULL, NULL, 0);
        rtWorldEnd();
    rtEnd();
}
