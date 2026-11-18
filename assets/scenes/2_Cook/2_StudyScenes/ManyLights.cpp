/*
 * ECE/CS 285 – HW3 Study Scene: ManyLights
 * A long hall lit by 64 small lamps, emissive spheres 4 cm across in an
 * 8 x 8 grid 2.6 m up, warm and cool in turn, over matte spheres and two
 * blocks; no other light, no ambient. cook samples every light at every
 * hit, 64 shadow rays per camera ray: the scene that motivates choosing
 * one light per sample (HW4's light selection; extra credit in HW3).
 * 960 x 540, 16 spp, stratified, seed 0.
 *
 * Output: renders/<integrator>/2_StudyScenes/ManyLights.png
 */

#include "rt.h"
#include "sceneutil.h"

void ManyLights(void)
{
    RtPoint eye = {0, 1.7f, 5}, lookat = {0, 0.9f, -8};
    RtVector up = {0, 1, 0};
    RtColor black = {0, 0, 0};
    RtMaterial floorMat = MATTE(0.6f, 0.58f, 0.55f);
    RtMaterial wallMat = MATTE(0.55f, 0.55f, 0.6f);
    RtMaterial white = MATTE(0.75f, 0.75f, 0.75f);
    const RtMaterial balls[4] = {MATTE(0.75f, 0.2f, 0.15f), MATTE(0.2f, 0.6f, 0.25f), MATTE(0.2f, 0.35f, 0.8f),
                                 MATTE(0.8f, 0.7f, 0.2f)};
    const RtMaterial warm = EMITTER(900, 620, 330), cool = EMITTER(380, 560, 900);

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("2_StudyScenes/ManyLights");
        rtCamera(eye, lookat, up, 45);
        rtPixelSamples(16);
        rtSampler("stratified");
        rtAmbient(black);
        rtBackground(black);
        rtWorldBegin(0);
            /* The hall: 8 m wide, 20 m long, 3.2 m high, open at the top
             * and toward the camera. */
            rtMaterial(&floorMat);
            Quad(Vec3(-4, 0, -18), Vec3(8, 0, 0), Vec3(0, 0, 20));     /* floor, facing up */
            rtMaterial(&wallMat);
            Quad(Vec3(-4, 3.2f, 2), Vec3(0, 0, -20), Vec3(0, -3.2f, 0)); /* left wall, facing +x */
            Quad(Vec3(4, 3.2f, -18), Vec3(0, 0, 20), Vec3(0, -3.2f, 0)); /* right wall, facing -x */
            Quad(Vec3(-4, 3.2f, -18), Vec3(8, 0, 0), Vec3(0, -3.2f, 0)); /* back wall, facing +z */

            /* The lamps, one rtSphere each, so 64 lights. */
            for (int i = 0; i < 8; i++) {
                for (int j = 0; j < 8; j++) {
                    rtMaterial((i + j) % 2 ? &cool : &warm);
                    rtTransformBegin();
                        rtTranslate(-3.15f + 0.9f * (float)i, 2.6f, 0.5f - 2.2f * (float)j);
                        rtSphere(0.02f);
                    rtTransformEnd();
                }
            }

            /* A row of spheres down the middle and two blocks. */
            for (int k = 0; k < 6; k++) {
                rtMaterial(&balls[k % 4]);
                rtTransformBegin();
                    rtTranslate(k % 2 ? 0.9f : -0.9f, 0.45f, -1.0f - 2.4f * (float)k);
                    rtSphere(0.45f);
                rtTransformEnd();
            }
            rtMaterial(&white);
            rtTransformBegin();
                rtTranslate(-2.6f, 0, -6);
                rtRotate(30, 0, 1, 0);
                Box(Vec3(-0.4f, 0, -0.4f), Vec3(0.4f, 1.6f, 0.4f));
            rtTransformEnd();
            rtTransformBegin();
                rtTranslate(2.6f, 0, -9);
                rtRotate(-20, 0, 1, 0);
                Box(Vec3(-0.5f, 0, -0.5f), Vec3(0.5f, 1.0f, 0.5f));
            rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}
