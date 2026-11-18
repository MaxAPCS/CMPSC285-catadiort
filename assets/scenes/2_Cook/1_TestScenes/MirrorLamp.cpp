/*
 * ECE/CS 285 – HW3 Test Scene: MirrorLamp
 * Two metal plates (specular 1) before a checker wall, the left perfectly
 * smooth, the right at roughness 0.2. Over each, a lamp (a glowing ball,
 * radius 3 cm, radiance 0.8) and, farther out, a point light, which has
 * no size and cannot be seen: a red bead marks it and lets its light
 * through. Tests what each plate takes from a light: the smooth plate
 * only what its mirror ray finds, the lamp at its own radiance and no
 * light at the bead; the rough plate both, as soft highlights.
 * 960 x 540, 64 spp, stratified, seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/MirrorLamp.png
 */

#include "rt.h"
#include "sceneutil.h"

/* The bead around a point light: seen by the camera and in reflections,
 * not by shadow rays, so the light inside it shines out. */
static RtInt BEAD(RtShadeContext *c) { return c->depth >= 0; }

void MirrorLamp(void)
{
    RtPoint eye = {0, 1.0f, 2.5f}, lookat = {0, 0.036f, -0.3f};
    RtVector up = {0, 1, 0};
    RtColor ka = {0.25f, 0.25f, 0.25f};
    RtColor black = {0, 0, 0};
    RtColor I = {0.008f, 0.008f, 0.008f}; /* 3.5 times the lamp's intensity, Le pi r^2 */
    RtMaterial lamp = EMITTER(0.8f, 0.8f, 0.8f);
    RtMaterial bead = MATTE(0.8f, 0.08f, 0.06f);
    RtMaterial smooth = {{0, 0, 0}, {1, 1, 1}, 0, 1, 0, 1.5f, {0, 0, 0}};
    RtMaterial rough = {{0, 0, 0}, {1, 1, 1}, 0.2f, 1, 0, 1.5f, {0, 0, 0}};
    RtMaterial dark = MATTE(0.1f, 0.1f, 0.1f);
    RtMaterial white = MATTE(0.8f, 0.8f, 0.8f);
    CheckerParams wall = GRAY_CHECKER(12);

    rtBegin();
        rtFormat(960, 540);
        rtDisplay("1_TestScenes/MirrorLamp");
        rtCamera(eye, lookat, up, 35);
        rtPixelSamples(64);
        rtSampler("stratified");
        rtAmbient(ka);
        rtBackground(black);
        rtWorldBegin(0);
            /* The lamps, 0.6 m over the plates, 0.35 m from the middle. */
            rtMaterial(&lamp);
            rtTransformBegin(); rtTranslate(-0.35f, 0.6f, -0.3f); rtSphere(0.03f); rtTransformEnd();
            rtTransformBegin(); rtTranslate(0.35f, 0.6f, -0.3f); rtSphere(0.03f); rtTransformEnd();

            /* The point lights, at the same height 0.85 m out, each in its bead. */
            RtPoint left = {-0.85f, 0.6f, -0.3f}, right = {0.85f, 0.6f, -0.3f};
            rtPointLight(left, I);
            rtPointLight(right, I);
            rtMaterial(&bead);
            rtCutout(BEAD, NULL, 0);
            rtTransformBegin(); rtTranslate(left[0], left[1], left[2]); rtSphere(0.015f); rtTransformEnd();
            rtTransformBegin(); rtTranslate(right[0], right[1], right[2]); rtSphere(0.015f); rtTransformEnd();
            rtCutout(NULL, NULL, 0);

            /* The plates, 1 m wide with a 10 cm gap, a hair above the floor. */
            rtMaterial(&smooth);
            Quad(Vec3(-1.05f, 0.001f, -1.2f), Vec3(1, 0, 0), Vec3(0, 0, 2.8f));
            rtMaterial(&rough);
            Quad(Vec3(0.05f, 0.001f, -1.2f), Vec3(1, 0, 0), Vec3(0, 0, 2.8f));
            rtMaterial(&dark);
            Floor(8);

            /* The wall the plates reflect, 1.5 m behind the lamps. */
            rtMaterial(&white);
            rtSurface(CHECKER, &wall, sizeof wall);
            Quad(Vec3(-4, 3, -1.8f), Vec3(8, 0, 0), Vec3(0, -3, 0));
            rtSurface(NULL, NULL, 0);
        rtWorldEnd();
    rtEnd();
}
