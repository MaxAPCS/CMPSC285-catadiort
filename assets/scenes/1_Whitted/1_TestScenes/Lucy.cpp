/*
 * ECE/CS 285 – HW2 Test Scene: Lucy
 * The Stanford Lucy, 1.6 m tall, half a million triangles with vertex
 * normals, in white marble on a pedestal of dark green marble, under a
 * key, a fill and no ambient. The marble is MARBLE (sceneutil.h), shaded
 * in object space so the veins are carved into the stone.
 * 720 x 1080; 256 spp, halton, seed 0 under kajiya (no cook reference).
 * Model: the Stanford Computer Graphics Laboratory's Lucy (28 million
 * triangles as scanned), reduced to 500,000; models/lucy/lucy.mesh.
 *
 * Output: renders/<integrator>/1_TestScenes/Lucy.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h" /* Fbm, Box, Floor, the materials */

#include <cmath>

void Lucy(void)
{
    RtPoint eye = {0.45f, 1.35f, 2.7f}, lookat = {0, 1.05f, 0};
    RtVector up = {0, 1, 0};
    RtPoint keyPos = {-3, 4.5f, 4}, fillPos = {3.5f, 2.5f, 3};
    RtColor keyI = {90, 90, 90}, fillI = {28, 28, 30}; /* W/sr */
    RtColor ka = {0, 0, 0};
    RtMaterial floor = MATTE(0.35f, 0.33f, 0.3f);
    /* polished stone: a weak white highlight, not a mirror (roughness
     * 0.45, alpha 0.2, is past whitted's mirror cutoff, so no mirror ray) */
    RtMaterial polished = {{0.8f, 0.8f, 0.8f}, {0.25f, 0.25f, 0.25f}, 0.45f, 0, 0, 1.5f, {0, 0, 0}};
    /* white Carrara-like marble: thin grey veins about 15 cm apart (the
     * scan's object space is in millimeters, so the scale is 150) */
    MarbleParams white = {150, 0.7f, 0.06f, 0.15f, {0.9f, 0.88f, 0.84f}, {0.34f, 0.34f, 0.4f}};
    /* dark green marble: pale veins, coarser and more tangled */
    MarbleParams green = {0.3f, 0.9f, 0.08f, 0.3f, {0.05f, 0.14f, 0.09f}, {0.62f, 0.64f, 0.56f}};

    rtBegin();
        rtFormat(720, 1080);
        rtDisplay("1_TestScenes/Lucy");
        rtPixelSamples(256); /* cook and kajiya; whitted renders one sample */
        rtSampler("halton");
        rtCamera(eye, lookat, up, 46);
        rtAmbient(ka);
        rtBackground(Color8(40, 44, 52));
        rtWorldBegin(0);
            rtPointLight(keyPos, keyI);
            rtPointLight(fillPos, fillI);

            rtMaterial(&floor);
            Floor(12);

            /* The pedestal: 0.7 m square, 0.4 m tall, in green marble. */
            rtMaterial(&polished);
            rtSurface(MARBLE, &green, sizeof green);
            Box(Vec3(-0.35f, 0, -0.35f), Vec3(0.35f, 0.4f, 0.35f));

            /* The scan is in millimeters with z up, 1,597 mm tall, its base
             * at z = -606 and its center at (691, -122): center it, stand it
             * on z = 0, scale it to meters, turn z up into y up, and turn
             * her to face the camera. OBJ_KEEP_MATERIAL: the scan has no
             * MTL, so the stone bound here is kept. */
            ObjMesh *lucy = obj_load("models/lucy/lucy.mesh");
            rtSurface(MARBLE, &white, sizeof white);
            rtTransformBegin();
                rtTranslate(0, 0.4f, 0);
                rtRotate(180, 0, 1, 0);
                rtRotate(-90, 1, 0, 0);
                rtScale(0.001f, 0.001f, 0.001f);
                rtTranslate(-691, 122, 606);
                ObjDraw(lucy, OBJ_KEEP_MATERIAL);
            rtTransformEnd();
            obj_free(lucy);
            rtSurface(NULL, NULL, 0);
        rtWorldEnd();
    rtEnd();
}
