/*
 * ECE/CS 285 – HW3 Test Scene: SphereLight and SphereLightInside
 * An emissive sphere (radius 0.15 m, radiance 150) 1.8 m over a grey
 * floor, in front of a matte red, a glossy blue (roughness 0.45) and a
 * mirror sphere; no other light, no ambient, black sky. Tests sampling a
 * sphere light in the cone it subtends. SphereLightInside: a lamp sphere
 * sunk into the floor around a small ball, hidden from the camera
 * (LAMP_SPHERE). Tests a point inside a lamp, lit by a disc of the lamp's
 * radius at its center, turned toward it.
 * 960 x 540, 64 spp, stratified, seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/SphereLight.png,
 *         SphereLightInside.png
 */

#include "rt.h"
#include "sceneutil.h"

#include <cmath>

void SphereLight(void)
{
    RtPoint eye = {0, 1.6f, 5.2f}, lookat = {0, 0.55f, 0};
    RtVector up = {0, 1, 0};
    RtColor black = {0, 0, 0};
    RtMaterial lamp = EMITTER(150, 150, 150);
    RtMaterial grey = MATTE(0.5f, 0.5f, 0.5f);
    RtMaterial red = MATTE(0.75f, 0.15f, 0.12f);
    RtMaterial glossy = {{0.08f, 0.2f, 0.55f}, {0.5f, 0.5f, 0.5f}, 0.45f, 0, 0, 1.5f, {0, 0, 0}};
    RtMaterial mirror = MIRROR;

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/SphereLight");
        rtCamera(eye, lookat, up, 40);
        rtPixelSamples(64);
        rtSampler("stratified");
        rtAmbient(black);
        rtBackground(black);
        rtWorldBegin(0);
            rtMaterial(&lamp);
            rtTransformBegin(); rtTranslate(0, 1.8f, 0.6f); rtSphere(0.15f); rtTransformEnd();

            rtMaterial(&grey);
            Floor(12);

            rtMaterial(&red);
            rtTransformBegin(); rtTranslate(-1.15f, 0.45f, -0.5f); rtSphere(0.45f); rtTransformEnd();
            rtMaterial(&glossy);
            rtTransformBegin(); rtTranslate(0, 0.45f, -0.7f); rtSphere(0.45f); rtTransformEnd();
            rtMaterial(&mirror);
            rtTransformBegin(); rtTranslate(1.15f, 0.45f, -0.5f); rtSphere(0.45f); rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}

/* The lamp: radius 0.6 m, its center 0.3 m over the floor, so it meets
 * the floor in a circle 0.52 m across; a matte ball inside it, off its
 * axis, and one outside, lit by the cone. */
void SphereLightInside(void)
{
    RtPoint eye = {0, 1.7f, 3.2f}, lookat = {0, 0.15f, 0};
    RtVector up = {0, 1, 0};
    RtColor black = {0, 0, 0};
    RtMaterial lamp = EMITTER(1.2f, 1.2f, 1.2f);
    RtMaterial grey = MATTE(0.5f, 0.5f, 0.5f);
    RtMaterial red = MATTE(0.75f, 0.15f, 0.12f);
    RtMaterial blue = MATTE(0.12f, 0.25f, 0.7f);

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/SphereLightInside");
        rtCamera(eye, lookat, up, 35);
        rtPixelSamples(64);
        rtSampler("stratified");
        rtAmbient(black);
        rtBackground(black);
        rtWorldBegin(0);
            rtMaterial(&lamp);
            rtCutout(LAMP_SPHERE, NULL, 0);
            rtTransformBegin(); rtTranslate(0, 0.3f, 0); rtSphere(0.6f); rtTransformEnd();
            rtCutout(NULL, NULL, 0);

            rtMaterial(&grey);
            Floor(12);

            rtMaterial(&red);
            rtTransformBegin(); rtTranslate(0.25f, 0.12f, 0.15f); rtSphere(0.12f); rtTransformEnd();
            rtMaterial(&blue);
            rtTransformBegin(); rtTranslate(-1.0f, 0.25f, -0.3f); rtSphere(0.25f); rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}
