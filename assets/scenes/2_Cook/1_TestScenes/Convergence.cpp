/*
 * ECE/CS 285 – HW3 Test Scene: Convergence
 * A glossy gold sphere on a checker floor under a square lamp, seen
 * through a lens focused on the sphere: the scene of the sampler study,
 * which tools/convergence.py renders with every sampler at 1 to 1024 spp
 * against Convergence_ref.pfm (16,384 spp, independent, --make-ref).
 * 320 x 180, 64 spp, independent, seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/Convergence.png
 */
#include "rt.h"
#include "sceneutil.h"

void Convergence(void)
{
    RtPoint eye = {0, 2.0f, 3.0f}, lookat = {0, 0.2f, 0};
    RtVector up = {0, 1, 0};
    RtColor black = {0, 0, 0};
    const float Le = 2.5f;
    RtMaterial lamp = EMITTER(Le, Le, Le);
    RtMaterial white = MATTE(0.75f, 0.75f, 0.75f);
    RtMaterial gold = {{0.35f, 0.22f, 0.05f}, {0.6f, 0.5f, 0.3f}, 0.5f, 0, 0, 1.5f, {0, 0, 0}};
    CheckerParams checks = {333, 333, MATTE(0.75f, 0.75f, 0.75f), MATTE(0.08f, 0.08f, 0.08f)}; /* 0.12 m */

    rtBegin();
        rtFormat(320, 180);
        rtDisplay("1_TestScenes/Convergence");
        rtCamera(eye, lookat, up, 30);
        rtLens(0.07f, 3.40f); /* focused on the sphere's center */
        rtPixelSamples(64);
        rtAmbient(black);
        rtWorldBegin(0);
            /* a 3 m square lamp 3 m up over the sphere, facing down */
            rtMaterial(&lamp);
            Quad(Vec3(-1.5f, 3.0f, 1.5f), Vec3(3.0f, 0, 0), Vec3(0, 0, -3.0f));

            /* the floor, 40 m across, so that it fills the frame */
            rtMaterial(&white);
            rtSurface(CHECKER, &checks, sizeof checks);
            Floor(40);
            rtSurface(NULL, NULL, 0);

            rtMaterial(&gold);
            rtTransformBegin();
                rtTranslate(0, 0.35f, 0);
                rtSphere(0.35f);
            rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}
