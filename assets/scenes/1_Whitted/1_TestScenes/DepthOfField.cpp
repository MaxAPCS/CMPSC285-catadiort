/*
 * ECE/CS 285 – HW2 Test Scene: DepthOfField
 * Eight matte spheres of radius 0.2 m, from 2 m to 12.5 m, 1.5 m apart,
 * in a row across a checkered floor (FocusSweep's row), seen through a
 * lens of radius 5 cm (rtLens(0.05, d)) focused on the nearest (_near),
 * the fifth (_mid) or the last (_far). Exactly one sphere is sharp, with
 * a straight, level band of sharp floor at its depth; the blur grows
 * with the distance from that plane, faster toward the camera. 960 x
 * 540, 64 spp, stratified, seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/DepthOfField_near.png,
 *         DepthOfField_mid.png, DepthOfField_far.png
 */

#include "rt.h"
#include "shaders.h"
#include "sceneutil.h"

#include <cmath>

static void depthOfField(int focusSphere, const char *display)
{
    const Vec3 eye(0, 0.45f, 0), lookat(0, -0.075f, -6); /* low, looking 5 degrees down */
    const Vec3 forward = normalize(lookat - eye);
    RtVector up = {0, 1, 0};
    RtVector sunDir = {-0.5f, -1, -0.35f};
    RtColor sunE = {3, 3, 3}; /* W/m^2 */
    RtColor ka = {0.25f, 0.25f, 0.25f};

    /* Depths 2, 3.5, ..., 12.5 m, each sphere 0.28 m further right than
     * the last, so the row runs from lower left to upper right, all of
     * radius 0.2 m: the row of FocusSweep (2_StudyScenes), which focuses
     * on every sphere in turn. */
    const int count = 8;
    Vec3 centers[count];
    float radii[count];
    for (int i = 0; i < count; i++) {
        radii[i] = 0.2f;
        centers[i] = Vec3(-0.6f + 0.28f * (float)i, radii[i], -(2.0f + 1.5f * (float)i));
    }
    const RtMaterial colors[count] = {MATTE(0.8f, 0.2f, 0.15f),  MATTE(0.85f, 0.5f, 0.1f), MATTE(0.75f, 0.75f, 0.2f),
                                      MATTE(0.35f, 0.7f, 0.2f),  MATTE(0.15f, 0.6f, 0.5f), MATTE(0.15f, 0.4f, 0.85f),
                                      MATTE(0.45f, 0.3f, 0.8f),  MATTE(0.75f, 0.3f, 0.6f)};

    /* The plane of focus goes through the chosen sphere's center. */
    const float focus = dot(centers[focusSphere] - eye, forward);

    RtPoint eyeP, lookatP;
    eye.store(eyeP);
    lookat.store(lookatP);
    CheckerParams checks = {48, 96, MATTE(0.75f, 0.72f, 0.65f), MATTE(0.22f, 0.22f, 0.25f)};
    RtMaterial white = MATTE(1, 1, 1);

    rtBegin();
        rtFormat(960, 540);
        rtDisplay(display);
        rtCamera(eyeP, lookatP, up, 25);
        rtLens(0.05f, focus);
        rtPixelSamples(64);
        rtSampler("stratified");
        rtAmbient(ka);
        rtBackground(Color(0.6f, 0.7f, 0.85f));
        rtWorldBegin(0);
            rtDirectionalLight(sunDir, sunE);

            /* 12 m across and 24 m deep in checks of 0.25 m */
            rtMaterial(&white);
            rtSurface(CHECKER, &checks, sizeof checks);
            Quad(Vec3(-6, 0, -22), Vec3(12, 0, 0), Vec3(0, 0, 24));
            rtSurface(NULL, NULL, 0);

            for (int i = 0; i < count; i++) {
                rtMaterial(&colors[i]);
                rtTransformBegin();
                    rtTranslate(centers[i].x, centers[i].y, centers[i].z);
                    rtSphere(radii[i]);
                rtTransformEnd();
            }
        rtWorldEnd();
    rtEnd();
}

void DepthOfField_near(void) { depthOfField(0, "1_TestScenes/DepthOfField_near"); }
void DepthOfField_mid(void) { depthOfField(4, "1_TestScenes/DepthOfField_mid"); }
void DepthOfField_far(void) { depthOfField(7, "1_TestScenes/DepthOfField_far"); }
