/*
 * ECE/CS 285 – HW2 Test Scene: GlossySweep
 * Five colored spheres, specular 0.5, roughness 0.224, 0.316, 0.447,
 * 0.592 and 0.707 (alpha = roughness^2) left to right, on a glossy
 * checkered floor under a 3 m by 2 m emissive quad. Each sphere samples
 * its Phong lobe (one ray per sample, HW3), so the quad's
 * reflection shows the lobe: sharp on the left, a broad smear of the same
 * total brightness on the right. Under whitted the mirror ray fades with
 * alpha (weight 1 - alpha / 0.15): two thirds, a third,
 * then none. 960 x 540, 256 spp, stratified, seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/GlossySweep.png
 */

#include "rt.h"
#include "shaders.h"
#include "sceneutil.h"

#include <cmath>

void GlossySweep(void)
{
    RtPoint eye = {0, 1.4f, 6.2f}, lookat = {0, 0.55f, 0};
    RtVector up = {0, 1, 0};
    RtColor ka = {0.03f, 0.03f, 0.03f};
    const float roughness[5] = {0.224f, 0.316f, 0.447f, 0.592f, 0.707f};
    RtMaterial light = EMITTER(5, 5, 5);
    RtMaterial white = MATTE(1, 1, 1);
    CheckerParams checks = {12, 12, {{0.45f, 0.45f, 0.43f}, {0.5f, 0.5f, 0.5f}, 0.387f, 0, 0, 1.5f, {0, 0, 0}},
                            {{0.08f, 0.08f, 0.1f}, {0.5f, 0.5f, 0.5f}, 0.387f, 0, 0, 1.5f, {0, 0, 0}}};

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/GlossySweep");
        rtCamera(eye, lookat, up, 35);
        rtPixelSamples(256);
        rtSampler("stratified");
        rtAmbient(ka);
        rtBackground(ka);
        rtWorldBegin(0);
            rtMaterial(&light);
            Quad(Vec3(-1.5f, 3.2f, 3), Vec3(3, 0, 0), Vec3(0, 0, -2)); /* facing down, over z = 1 to 3 */

            rtMaterial(&white);
            rtSurface(CHECKER, &checks, sizeof checks);
            Floor(12); /* 1 m checks */
            rtSurface(NULL, NULL, 0);

            const float colors[5][3] = {{0.4f, 0.07f, 0.05f}, {0.42f, 0.28f, 0.04f}, {0.08f, 0.32f, 0.1f},
                                        {0.06f, 0.16f, 0.42f}, {0.28f, 0.08f, 0.38f}};
            for (int i = 0; i < 5; i++) {
                RtMaterial m = {{colors[i][0], colors[i][1], colors[i][2]}, {0.5f, 0.5f, 0.5f}, roughness[i], 0, 0,
                                1.5f, {0, 0, 0}};
                rtMaterial(&m);
                rtTransformBegin();
                    rtTranslate(-2.4f + 1.2f * (float)i, 0.5f, 0);
                    rtSphere(0.5f);
                rtTransformEnd();
            }
        rtWorldEnd();
    rtEnd();
}
