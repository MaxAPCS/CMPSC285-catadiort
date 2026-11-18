/*
 * ECE/CS 285 – HW3 Test Scene: FrostedEarth
 * A glass globe of the Earth on a checker floor in front of a checker
 * wall, under a square lamp. The scene's GLOBE shader reads earth.png
 * and leaves the oceans clear (roughness 0) and frosts the land
 * (roughness 0.25): the wall seen through the water is sharp and through
 * the continents blurred, glossy transmission, the refracted ray spread
 * into the lobe like the reflected one. Also a movie: the globe turns
 * eastward once over 71 frames at 24 a second, about 3 s (a 72nd frame
 * would be the first again, so the movie loops without a seam; the
 * scene's argument, gauchoRT FrostedEarthMovie 2 for 2 turns).
 * FrostedEarth renders frame 0 at 256 spp, FrostedEarthMovie all 71 at
 * 64; 960 x 540, halton, seed 0. Make the movie with
 *   ffmpeg -framerate 24 -i renders/2_Cook/1_TestScenes/FrostedEarthMovie/FrostedEarthMovie_%03d.png
 *          -c:v libx264 -pix_fmt yuv420p -crf 18
 *          renders/2_Cook/1_TestScenes/FrostedEarthMovie.mp4
 *
 * Output: renders/<integrator>/1_TestScenes/FrostedEarth.png, and
 *         FrostedEarthMovie/FrostedEarthMovie_000.png to _070.png
 */

#include "rt.h"
#include "sceneutil.h"

#include <cstdio>

/* Clear glass over the oceans, frosted over the land: the roughness from
 * earth.png, water where blue leads the other channels by 0.05 (the
 * texture is read as stored, not linearized: it is a mask here). */
typedef struct {
    RtInt earth;
    RtFloat frost;
} GlobeParams;

static void GLOBE(RtShadeContext *c)
{
    const GlobeParams *p = (const GlobeParams *)c->params;
    RtFloat rgba[4];
    rtTexture(p->earth, c->u, c->v, rgba);
    const int water = rgba[2] > rgba[0] + 0.05f && rgba[2] > rgba[1] + 0.05f;
    c->m.roughness = water ? 0 : p->frost;
}

/* The scene at frame `frame` of the globe's turn (the still: frame 0,
 * the Americas facing the camera), or every frame of the movie when
 * frame is negative; frame f turns the globe 360 turns f / 71 degrees
 * eastward (turns is the default; the scene's first argument overrides
 * it), so frame 71 would be frame 0 again and the movie loops. Each frame is a world of its own (rtWorldBegin/End) inside one
 * rtBegin/End, named with rtDisplay. */
static void FrostedEarthScene(int frame, float turns, int spp)
{
    const int frames = 71; /* one turn, about 3 s at 24 frames a second */
    RtPoint eye = {0, 1.9f, 6.5f}, lookat = {0, 1.3f, 0};
    RtVector up = {0, 1, 0};
    RtColor ka = {0.1f, 0.1f, 0.1f};
    RtColor sky = {0.12f, 0.12f, 0.14f};
    RtMaterial lamp = EMITTER(14, 14, 14);
    RtMaterial glass = GLASS;
    RtMaterial white = MATTE(0.8f, 0.8f, 0.8f);
    CheckerParams floor = GRAY_CHECKER(16);                                        /* 1 m checks */
    CheckerParams wall = {16, 8, MATTE(Color8(235, 235, 235)), MATTE(Color8(120, 120, 120))}; /* 0.75 m */

    rtBegin();
        rtFormat(960, 540);
        rtPixelSamples(spp);
        rtSampler("halton");
        rtCamera(eye, lookat, up, 32);
        rtAmbient(ka);
        rtBackground(sky);
        for (int f = (frame < 0 ? 0 : frame); f < frames; f++) {
            char name[96];
            if (frame >= 0) {
                snprintf(name, sizeof name, "1_TestScenes/FrostedEarth");
            } else {
                snprintf(name, sizeof name, "1_TestScenes/FrostedEarthMovie/FrostedEarthMovie_%03d", f);
            }
            rtDisplay(name);
            if (rtWorldBegin(frame >= 0 ? 0 : f)) { /* --frames may leave a frame out */
                /* The lamp: a 1.2 m square 3.6 m up, in front of the globe, facing down. */
                rtMaterial(&lamp);
                Quad(Vec3(-0.6f, 3.6f, 1.8f), Vec3(0, 0, 1.2f), Vec3(1.2f, 0, 0));

                /* The checker floor, and the wall 3 m behind the globe facing the camera. */
                rtMaterial(&white);
                rtSurface(CHECKER, &floor, sizeof floor);
                Floor(16);
                rtSurface(CHECKER, &wall, sizeof wall);
                Quad(Vec3(-6, 6, -3), Vec3(12, 0, 0), Vec3(0, -6, 0));
                rtSurface(NULL, NULL, 0);

                /* The globe: radius 1.3 m, resting on the floor, turned -80
                 * degrees about y so the Americas face the camera at frame 0
                 * (as in Earth), and on eastward from there. */
                rtMaterial(&glass);
                GlobeParams globe = {rtMakeTexture("earth.png", 0), 0.25f};
                rtSurface(GLOBE, &globe, sizeof globe);
                rtTransformBegin();
                    rtTranslate(0, 1.3f, 0);
                    rtRotate(-80 + 360.0f * rtSceneArg(0, turns) * (float)f / (float)frames, 0, 1, 0);
                    rtSphere(1.3f);
                rtTransformEnd();
                rtSurface(NULL, NULL, 0);
                rtWorldEnd();
            }
            if (frame >= 0) {
                break;
            }
        }
    rtEnd();
}

void FrostedEarth(void) { FrostedEarthScene(0, 1, 256); }
void FrostedEarthMovie(void) { FrostedEarthScene(-1, 1, 64); }
