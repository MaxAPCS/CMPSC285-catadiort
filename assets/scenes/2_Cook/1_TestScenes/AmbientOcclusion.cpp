/*
 * ECE/CS 285 – HW3 Test Scene: AmbientOcclusion
 * The Stanford dragon, white and matte, in a closed white room with no
 * lights: only the ambient term, ka = 0.6, scaled by ambient occlusion
 * within half a meter (rtAmbientOcclusion(0.5)), one cosine-weighted ray
 * per sample. Tests the occlusion term alone.
 * 960 x 540, 64 spp, stratified, seed 0.
 * Model: the Stanford dragon (Curless and Levoy, 1996), models/dragon/.
 *
 * Output: renders/<integrator>/1_TestScenes/AmbientOcclusion.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

void AmbientOcclusion(void)
{
    RtPoint eye = {-0.9f, 1.3f, 3.3f}, lookat = {0.1f, 0.6f, 0};
    RtVector up = {0, 1, 0};
    RtColor ka = {0.6f, 0.6f, 0.6f};
    RtColor black = {0, 0, 0};
    RtMaterial white = MATTE(0.8f, 0.8f, 0.8f);

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/AmbientOcclusion");
        rtCamera(eye, lookat, up, 45);
        rtPixelSamples(64);
        rtSampler("stratified");
        rtAmbient(ka);
        rtAmbientOcclusion(0.5f);
        rtBackground(black);
        rtWorldBegin(0);
            rtMaterial(&white);
            Box(Vec3(-2.5f, 0, -1.6f), Vec3(2.5f, 2.6f, 3.8f), true); /* the room, seen from inside */

            /* As in the Dragon benchmark: the scan is 0.144 m tall with its
             * feet at y = 0.053; scaled by 10 it stands 1.44 m. */
            ObjMesh *dragon = obj_load("models/dragon/dragon.mesh");
            rtTransformBegin();
                rtRotate(-20, 0, 1, 0);
                rtScale(10, 10, 10);
                rtTranslate(0.006f, -0.053f, 0.005f);
                ObjDraw(dragon, OBJ_KEEP_MATERIAL);
            rtTransformEnd();
            obj_free(dragon);
        rtWorldEnd();
    rtEnd();
}
