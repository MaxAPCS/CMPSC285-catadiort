/*
 * ECE/CS 285 – HW2 Test Scene: FunWithRefraction
 * A red, a green and a blue sphere behind a glass ball, over a glass slab
 * with a pink rod through it; one point light and a sun: refraction with
 * a Fresnel rim, the rod bent in the slab, total internal reflection in
 * the slab's bottom face. Also a movie: the spheres circle once about the
 * line of sight behind the ball 4 times in 96 frames at 24 a second (the
 * speed is the scene's argument), each frame exposed over half its turn
 * (cook blurs them, whitted does not); the rest stays put.
 * FunWithRefraction renders frame 0, FunWithRefractionMovie all 96.
 * 720 x 720, 64 spp, halton under cook. Make the movie with
 *   ffmpeg -framerate 24 -i renders/1_Whitted/1_TestScenes/FunWithRefractionMovie/FunWithRefractionMovie_%03d.png
 *          -c:v libx264 -pix_fmt yuv420p -crf 18
 *          renders/1_Whitted/1_TestScenes/FunWithRefractionMovie.mp4
 *
 * Output: renders/<integrator>/1_TestScenes/FunWithRefraction.png, and
 *         FunWithRefractionMovie/FunWithRefractionMovie_000.png to _095.png
 */

#include "rt.h"
#include "sceneutil.h"

#include <cstdio>

/* The scene at one frame of the spheres' turn, or every frame when frame
 * is negative. Frame f turns them by 360 f / 96 degrees, so frame 0 is
 * the still. */
static void FunWithRefractionScene(int frame)
{
    const int frames = 96;
    RtPoint eye = {3.84789f, 3.48111f, 14.0946f}, lookat = {3.63803f, 3.46678f, 13.117f};
    RtVector up = {-0.0559f, 0.998433f, -0.0026367f};
    RtColor ka = {0.025f, 0.025f, 0.025f};
    RtPoint light0Pos = {0.688645f, 7.68217f, 2.30317f};
    RtColor light0I = {51.3704f, 51.3704f, 51.3704f}; /* W/sr: 0.751 pi * (1.0f, 1.0f, 1.0f) at 4.67 m, the focal point */
    RtVector sun0Dir = {-0.209856f, -0.0143311f, -0.977627f};
    RtColor sun0E = {2.35934f, 2.35934f, 2.35934f}; /* W/m^2: 0.751 pi * (1.0f, 1.0f, 1.0f) */
    RtMaterial m0 = {{0.05812f, 0.5f, 0.0f}, {0.0f, 0.0f, 0.0f}, 0.269191f, 0, 0.0f, 1.5f, {0.0f, 0.0f, 0.0f}};
    RtMaterial m1 = {{0.47f, 0.0f, 0.0209248f}, {0.0f, 0.0f, 0.0f}, 0.269191f, 0, 0.0f, 1.5f, {0.0f, 0.0f, 0.0f}};
    RtMaterial m2 = {{0.0513605f, 0.025668f, 0.5f}, {0.0f, 0.0f, 0.0f}, 0.269191f, 0, 0.0f, 1.5f, {0.0f, 0.0f, 0.0f}};
    RtMaterial m3 = {{0.194609f, 0.192348f, 0.204082f}, {1.0f, 1.0f, 1.0f}, 0.139522f, 0, 0.85f, 1.5f, {0.0f, 0.0f, 0.0f}};
    RtMaterial m4 = {{0.045f, 0.055f, 0.08f}, {0.5f, 0.5f, 0.5f}, 0.13211f, 0, 0.85f, 1.5f, {0.0f, 0.0f, 0.0f}};
    static const RtPoint ps0[36] = {
        {-1.19112f, 1.43378f, 2.08234f}, {-1.19083f, -0.493891f, 1.54934f}, {3.15848f, -0.357657f, 1.05904f},
        {-1.19112f, 1.43378f, 2.08234f}, {3.15848f, -0.357657f, 1.05904f}, {3.15819f, 1.57001f, 1.59204f},
        {-1.5812f, 2.32223f, -1.13108f}, {2.76811f, 2.45846f, -1.62138f}, {2.76841f, 0.530794f, -2.15438f},
        {-1.5812f, 2.32223f, -1.13108f}, {2.76841f, 0.530794f, -2.15438f}, {-1.58091f, 0.39456f, -1.66408f},
        {-1.19112f, 1.43378f, 2.08234f}, {-1.5812f, 2.32223f, -1.13108f}, {-1.58091f, 0.39456f, -1.66408f},
        {-1.19112f, 1.43378f, 2.08234f}, {-1.58091f, 0.39456f, -1.66408f}, {-1.19083f, -0.493891f, 1.54934f},
        {3.15819f, 1.57001f, 1.59204f}, {3.15848f, -0.357657f, 1.05904f}, {2.76841f, 0.530794f, -2.15438f},
        {3.15819f, 1.57001f, 1.59204f}, {2.76841f, 0.530794f, -2.15438f}, {2.76811f, 2.45846f, -1.62138f},
        {-1.19112f, 1.43378f, 2.08234f}, {3.15819f, 1.57001f, 1.59204f}, {2.76811f, 2.45846f, -1.62138f},
        {-1.19112f, 1.43378f, 2.08234f}, {2.76811f, 2.45846f, -1.62138f}, {-1.5812f, 2.32223f, -1.13108f},
        {-1.19083f, -0.493891f, 1.54934f}, {-1.58091f, 0.39456f, -1.66408f}, {2.76841f, 0.530794f, -2.15438f},
        {-1.19083f, -0.493891f, 1.54934f}, {2.76841f, 0.530794f, -2.15438f}, {3.15848f, -0.357657f, 1.05904f},
    };
    RtMaterial m5 = {{0.8f, 0.163289f, 0.445193f}, {0.0f, 0.0f, 0.0f}, 0.269191f, 0, 0.0f, 1.5f, {0.0f, 0.0f, 0.0f}};
    static const RtPoint ps1[36] = {
        {-3.29592f, -1.81081f, -0.385227f}, {-3.13645f, -1.96867f, -0.527259f}, {3.01787f, 2.91434f, 0.955311f},
        {-3.29592f, -1.81081f, -0.385227f}, {3.01787f, 2.91434f, 0.955311f}, {2.8584f, 3.0722f, 1.09734f},
        {-3.3534f, -1.6719f, -0.604141f}, {2.80093f, 3.21111f, 0.878429f}, {2.96039f, 3.05325f, 0.736397f},
        {-3.3534f, -1.6719f, -0.604141f}, {2.96039f, 3.05325f, 0.736397f}, {-3.19393f, -1.82976f, -0.746174f},
        {-3.29592f, -1.81081f, -0.385227f}, {-3.3534f, -1.6719f, -0.604141f}, {-3.19393f, -1.82976f, -0.746174f},
        {-3.29592f, -1.81081f, -0.385227f}, {-3.19393f, -1.82976f, -0.746174f}, {-3.13645f, -1.96867f, -0.527259f},
        {2.8584f, 3.0722f, 1.09734f}, {3.01787f, 2.91434f, 0.955311f}, {2.96039f, 3.05325f, 0.736397f},
        {2.8584f, 3.0722f, 1.09734f}, {2.96039f, 3.05325f, 0.736397f}, {2.80093f, 3.21111f, 0.878429f},
        {-3.29592f, -1.81081f, -0.385227f}, {2.8584f, 3.0722f, 1.09734f}, {2.80093f, 3.21111f, 0.878429f},
        {-3.29592f, -1.81081f, -0.385227f}, {2.80093f, 3.21111f, 0.878429f}, {-3.3534f, -1.6719f, -0.604141f},
        {-3.13645f, -1.96867f, -0.527259f}, {-3.19393f, -1.82976f, -0.746174f}, {2.96039f, 3.05325f, 0.736397f},
        {-3.13645f, -1.96867f, -0.527259f}, {2.96039f, 3.05325f, 0.736397f}, {3.01787f, 2.91434f, 0.955311f},
    };

    /* the center of the three colored spheres, which they circle */
    const float cx = 0.0237f, cy = 5.3730f, cz = -3.7714f;

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
            char name[80];
            if (frame >= 0)
                snprintf(name, sizeof name, "1_TestScenes/FunWithRefraction");
            else
                snprintf(name, sizeof name, "1_TestScenes/FunWithRefractionMovie/FunWithRefractionMovie_%03d", f);
            rtDisplay(name);
            if (!rtWorldBegin(f)) { /* --frames left this frame out */
                continue;
            }
                rtPointLight(light0Pos, light0I);
                rtDirectionalLight(sun0Dir, sun0E);

                /* The turn: about the line of sight (lookat - eye, which
                 * rtRotate normalizes) through the spheres' center, 4
                 * turns over the movie unless the scene's first argument
                 * says otherwise (gauchoRT FunWithRefractionMovie 8), keyed
                 * at the frame's start and SHUTTER of a frame later, in
                 * its own transform block so it touches only them and
                 * does not carry over to the next frame. */
                rtTransformBegin();
                for (int key = 0; key < 2; key++) {
                    rtMotionBegin((float)key);
                    rtTranslate(cx, cy, cz);
                    rtRotate(360.0f * rtSceneArg(0, 4) * ((float)f + SHUTTER * (float)key) / (float)frames, lookat[0] - eye[0], lookat[1] - eye[1], lookat[2] - eye[2]);
                    rtTranslate(-cx, -cy, -cz);
                    rtMotionEnd();
                }

                rtMaterial(&m0);
                rtTransformBegin(); rtTranslate(-1.21548f, 4.09992f, -3.67961f); rtSphere(1.89935f); rtTransformEnd();
                rtMaterial(&m1);
                {
                    RtMatrix M = {{1.65772f, 0.0f, 0.0f, 0.0f}, {0.0f, 1.65772f, 0.0f, 0.0f}, {0.0f, 0.0f, 1.80818f, 0.0f}, {-0.461794f, 7.0127f, -4.00692f, 1.0f}}; /* an ellipsoid: its axes as rows */
                    rtTransformBegin(); rtConcatTransform(M); rtSphere(1.0f); rtTransformEnd();
                }
                rtMaterial(&m2);
                rtTransformBegin(); rtTranslate(1.74842f, 5.00647f, -3.62752f); rtSphere(1.72872f); rtTransformEnd();
                rtTransformEnd();

                rtMaterial(&m3);
                rtTransformBegin(); rtTranslate(1.36383f, 4.66175f, 3.35214f); rtSphere(1.36776f); rtTransformEnd();
                rtMaterial(&m4);
                Triangles(ps0, 12);
                rtMaterial(&m5);
                Triangles(ps1, 12);
            rtWorldEnd();
        }
    rtEnd();
}

void FunWithRefraction(void) { FunWithRefractionScene(0); }
void FunWithRefractionMovie(void) { FunWithRefractionScene(-1); }
