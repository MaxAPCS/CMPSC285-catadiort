/*
 * ECE/CS 285 – HW2 Test Scene: Bunny
 * The Stanford bunny (69,451 triangles, vertex normals, scaled by 11) on
 * a CHECKER floor next to a mirror sphere: interpolated normals and the
 * BVH (brute force 129 s on 16 threads, the BVH 0.08 s). 960 x 540; 256
 * spp, halton, seed 0 under cook and kajiya.
 * Model: the Stanford Computer Graphics Laboratory's bunny (Turk and
 * Levoy, 1994), models/bunny/bunny.mesh, from bun_zipper.ply.
 *
 * Output: renders/<integrator>/1_TestScenes/Bunny.png
 */

#include "rt.h"
#include "obj.h"
#include "shaders.h"
#include "sceneutil.h"

void Bunny(void)
{
    RtPoint eye = {0, 2.0f, 6.5f}, lookat = {0, 1.0f, 0};
    RtVector up = {0, 1, 0};
    RtPoint keyPos = {-4, 6, 5}, fillPos = {4, 5, 2};
    RtColor keyI = {110, 110, 110}, fillI = {45, 45, 45}; /* W/sr */
    RtColor ka = {0.05f, 0.05f, 0.05f};
    CheckerParams checks = GRAY_CHECKER(12);
    RtMaterial fur = MATTE(0.8f, 0.7f, 0.55f);
    RtMaterial mirror = MIRROR;

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/Bunny");
        rtPixelSamples(256);
        rtSampler("halton");
        rtCamera(eye, lookat, up, 36);
        rtAmbient(ka);
        rtWorldBegin(0);
            rtPointLight(keyPos, keyI);
            rtPointLight(fillPos, fillI);

            rtSurface(CHECKER, &checks, sizeof checks);
            Floor(12);
            rtSurface(NULL, NULL, 0);

            /* The scan is 0.154 m tall with its feet at y = 0.033 and its
             * center a little off the origin; scale it to 1.7 m and stand
             * it on the floor. OBJ_KEEP_MATERIAL: the scan has no MTL. */
            ObjMesh *bunny = obj_load("models/bunny/bunny.mesh");
            rtMaterial(&fur);
            rtTransformBegin();
                rtTranslate(-0.7f, 0, 1.0f);
                rtRotate(25, 0, 1, 0);
                rtScale(11, 11, 11);
                rtTranslate(0.017f, -0.033f, 0.0016f);
                ObjDraw(bunny, OBJ_KEEP_MATERIAL);
            rtTransformEnd();
            obj_free(bunny);

            rtMaterial(&mirror);
            rtTransformBegin(); rtTranslate(1.7f, 0.9f, -0.6f); rtSphere(0.9f); rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}
