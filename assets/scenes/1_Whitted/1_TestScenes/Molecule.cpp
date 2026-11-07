/*
 * ECE/CS 285 – HW2 Test Scene: Molecule
 * Seven spheres: a big grey one with three glossy colored ones around it
 * and three small matte ones beyond, one point light, black background:
 * highlights, mirror images between spheres, shadows. Also a movie: the
 * molecule turns about the line of sight through the big sphere, 96
 * frames at 24 a second, under a light that stays put, 4 turns over the
 * movie (15 degrees a frame; the scene's argument, gauchoRT MoleculeMovie
 * 8 for 8), each frame exposed over half its step, so cook blurs the
 * outer spheres a little where whitted steps. Molecule renders frame 0,
 * MoleculeMovie all 96. 720 x 720, 64 spp, halton under cook. Make the
 * movie with
 *   ffmpeg -framerate 24 -i renders/1_Whitted/1_TestScenes/MoleculeMovie/MoleculeMovie_%03d.png
 *          -c:v libx264 -pix_fmt yuv420p -crf 18
 *          renders/1_Whitted/1_TestScenes/MoleculeMovie.mp4
 *
 * Output: renders/<integrator>/1_TestScenes/Molecule.png, and
 *         MoleculeMovie/MoleculeMovie_000.png to MoleculeMovie_095.png
 */

#include "rt.h"
#include "sceneutil.h"

#include <cstdio>

/* The molecule at one frame of its turn, or every frame of the movie
 * named movie in the folder when frame is negative; frame f turns it by
 * 360 turns f / 96 degrees (turns is the default; the scene's first
 * argument overrides it, gauchoRT MoleculeMovie 8) and is exposed over
 * shutter of a step. A scene
 * that renders several images makes one world per image
 * (rtWorldBegin/End) inside one rtBegin/End, named with rtDisplay; the
 * options carry over. Not static: HW3's extra-credit movie draws the same
 * molecule faster. */
void MoleculeScene(int frame, const char *folder, const char *movie, float turns, float shutter)
{
    const int frames = 96;
    RtPoint eye = {5.26211f, 3.09263f, -1.38722f}, lookat = {4.40907f, 2.70908f, -1.03337f};
    RtVector up = {0.345784f, 0.0924084f, 0.933753f};
    RtColor ka = {0.025f, 0.025f, 0.025f};
    RtPoint light0Pos = {5.21898f, 4.68627f, -2.04877f};
    RtColor light0I = {28.3365f, 28.3365f, 28.3365f}; /* W/sr: 0.196 pi * (1.0f, 1.0f, 1.0f) at 6.78 m, the focal point */
    RtMaterial m0 = {{0.2f, 0.6f, 0.75f}, {0.0f, 0.0f, 0.0f}, 0.269191f, 0, 0.0f, 1.5f, {0.0f, 0.0f, 0.0f}};
    RtMaterial m1 = {{0.8f, 0.2f, 0.5f}, {0.0f, 0.0f, 0.0f}, 0.269191f, 0, 0.0f, 1.5f, {0.0f, 0.0f, 0.0f}};
    RtMaterial m2 = {{0.5f, 0.75f, 0.2f}, {0.0f, 0.0f, 0.0f}, 0.269191f, 0, 0.0f, 1.5f, {0.0f, 0.0f, 0.0f}};
    RtMaterial m3 = {{0.34f, 0.07f, 0.56f}, {0.4f, 0.4f, 0.4f}, 0.126962f, 0, 0.0f, 1.5f, {0.0f, 0.0f, 0.0f}};
    RtMaterial m4 = {{0.56f, 0.24f, 0.12f}, {0.4f, 0.4f, 0.4f}, 0.127573f, 0, 0.0f, 1.5f, {0.0f, 0.0f, 0.0f}};
    RtMaterial m5 = {{0.04f, 0.56f, 0.28f}, {0.4f, 0.4f, 0.4f}, 0.125767f, 0, 0.0f, 1.5f, {0.0f, 0.0f, 0.0f}};
    RtMaterial m6 = {{0.6f, 0.6f, 0.6f}, {0.5f, 0.5f, 0.5f}, 0.128821f, 0, 0.0f, 1.5f, {0.0f, 0.0f, 0.0f}};

    rtBegin();
        rtFormat(720, 720);
        rtCamera(eye, lookat, up, 45.0f);
        rtPixelSamples(64); /* whitted takes one; cook needs these for the blur */
        rtSampler("halton");
        rtAmbient(ka);
        rtMaxDepth(10);
        for (int f = 0; f < frames; f++) {
            if (frame >= 0 && f != frame)
                continue;
            char name[64];
            if (frame >= 0)
                snprintf(name, sizeof name, "%s/Molecule", folder);
            else
                snprintf(name, sizeof name, "%s/%s/%s_%03d", folder, movie, movie, f);
            rtDisplay(name);
            if (!rtWorldBegin(f)) { /* --frames left this frame out */
                continue;
            }
                rtPointLight(light0Pos, light0I);

                /* The turn: about the line of sight (lookat - eye, which
                 * rtRotate normalizes) through the big sphere's center
                 * (the last sphere below), so the molecule turns in the
                 * picture's plane, clockwise as seen. Keyed at the
                 * frame's start and SHUTTER of a frame later, so it moves
                 * over the exposure. It sits in its own transform block
                 * so it does not carry over to the next frame; the
                 * light, drawn before it, stays put. */
                rtTransformBegin();
                for (int key = 0; key < 2; key++) {
                    rtMotionBegin((float)key);
                    rtTranslate(0.0579858f, 0.496598f, 0.550744f);
                    rtRotate(360.0f * rtSceneArg(0, turns) * ((float)f + shutter * (float)key) / (float)frames, lookat[0] - eye[0], lookat[1] - eye[1], lookat[2] - eye[2]);
                    rtTranslate(-0.0579858f, -0.496598f, -0.550744f);
                    rtMotionEnd();
                }

                rtMaterial(&m0);
                rtTransformBegin(); rtTranslate(1.02616f, -0.365439f, -1.13081f); rtSphere(0.288315f); rtTransformEnd();
                rtMaterial(&m1);
                rtTransformBegin(); rtTranslate(7.4119e-08f, 2.78629f, -0.116676f); rtSphere(0.310878f); rtTransformEnd();
                rtMaterial(&m2);
                rtTransformBegin(); rtTranslate(1.39952f, 0.365459f, 2.25247f); rtSphere(0.312566f); rtTransformEnd();
                rtMaterial(&m3);
                rtTransformBegin(); rtTranslate(-3.21911e-07f, 2.01869f, 0.108636f); rtSphere(0.670496f); rtTransformEnd();
                rtMaterial(&m4);
                rtTransformBegin(); rtTranslate(0.990468f, 0.489687f, 1.61544f); rtSphere(0.636615f); rtTransformEnd();
                rtMaterial(&m5);
                rtTransformBegin(); rtTranslate(0.709821f, -0.124101f, -0.637882f); rtSphere(0.597582f); rtTransformEnd();
                rtMaterial(&m6);
                rtTransformBegin(); rtTranslate(0.0579858f, 0.496598f, 0.550744f); rtSphere(1.18902f); rtTransformEnd();
                rtTransformEnd();
            rtWorldEnd();
        }
    rtEnd();
}

void Molecule(void) { MoleculeScene(0, "1_TestScenes", "MoleculeMovie", 4, SHUTTER); }
void MoleculeMovie(void) { MoleculeScene(-1, "1_TestScenes", "MoleculeMovie", 4, SHUTTER); }
