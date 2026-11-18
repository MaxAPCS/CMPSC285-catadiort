/*
 * ECE/CS 285 – HW3 Study Scene: FocusSweep
 * Eight matte spheres of radius 0.2 m from 2 m to 12.5 m, 1.5 m apart,
 * in a row across a checkered floor, rendered eight times through a
 * lens of radius 8 cm (rtLens(0.08, d)) focused on each sphere in turn:
 * in frame k sphere k alone is sharp, and the blur grows in front of
 * and behind it, faster toward the camera. The row is DepthOfField's,
 * which focuses on its near, middle and far spheres only. Eight frames
 * at 480 x 270, 64 spp, stratified, seed 0. Make the strip, two rows of
 * four, with
 *   ffmpeg -i renders/2_Cook/2_StudyScenes/FocusSweep/FocusSweep_%03d.png
 *          -vf tile=4x2 renders/2_Cook/2_StudyScenes/FocusSweep.png
 *
 * Output: renders/<integrator>/2_StudyScenes/FocusSweep/FocusSweep_000.png
 *         to FocusSweep_007.png
 */

#include "rt.h"
#include "sceneutil.h"

#include <cstdio>

void FocusSweep(void)
{
    const Vec3 eye(0, 0.45f, 0), lookat(0, -0.075f, -6); /* low, looking 5 degrees down */
    const Vec3 forward = normalize(lookat - eye);
    RtVector up = {0, 1, 0};
    RtVector sunDir = {-0.5f, -1, -0.35f};
    RtColor sunE = {3, 3, 3}; /* W/m^2 */
    RtColor ka = {0.25f, 0.25f, 0.25f};

    /* Depths 2, 3.5, ..., 12.5 m, each sphere 0.28 m further right than
     * the last, so the row runs from lower left to upper right; all of
     * radius 0.2 m (the same row as DepthOfField). */
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

    RtPoint eyeP, lookatP;
    eye.store(eyeP);
    lookat.store(lookatP);
    CheckerParams checks = {48, 96, MATTE(0.75f, 0.72f, 0.65f), MATTE(0.22f, 0.22f, 0.25f)};
    RtMaterial white = MATTE(1, 1, 1);

    rtBegin();
        rtFormat(480, 270);
        rtCamera(eyeP, lookatP, up, 25);
        rtPixelSamples(64);
        rtSampler("stratified");
        rtAmbient(ka);
        rtBackground(Color(0.6f, 0.7f, 0.85f));
        /* One world per frame, the plane of focus through sphere k's center. */
        for (int k = 0; k < count; k++) {
            char name[64];
            snprintf(name, sizeof name, "2_StudyScenes/FocusSweep/FocusSweep_%03d", k);
            rtDisplay(name);
            rtLens(0.08f, dot(centers[k] - eye, forward));
            if (!rtWorldBegin(k)) { /* --frames left this frame out */
                continue;
            }
                rtDirectionalLight(sunDir, sunE);

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
        }
    rtEnd();
}
