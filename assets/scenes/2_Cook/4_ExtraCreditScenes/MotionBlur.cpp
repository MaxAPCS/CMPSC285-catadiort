/*
 * ECE/CS 285 – HW3 Extra Credit Scenes: motion blur
 * The scenes of the motion blur extra credit (rtMotionBegin/End): a ball,
 * a block and the camera keyed at two times (MotionBlur, MotionBlurPan),
 * Cook, Porter and Carpenter's pool table (1984), and HW2's molecule
 * spinning ten times over its movie (MoleculeFastMovie). A renderer
 * without motion blur draws each at its first pose. Settings per scene.
 * Model: the pool scene from Hachisuka et al. [2008] (github.com/
 * meistdan/pmdas, SDK/data/pool), for non-commercial use with this credit.
 *
 * Output: renders/<integrator>/4_ExtraCreditScenes/MotionBlur.png, MotionBlurPan.png,
 *         1984.png, MoleculeFastMovie/MoleculeFastMovie_000.png to _095.png
 */

#include "rt.h"
#include "shaders.h"
#include "sceneutil.h"

#include <cmath>

/* MotionBlur, MotionBlurPan -------------------------------------------- */

/* A red glossy ball sliding 1.4 m right over the shutter, a yellow block
 * turning 60 degrees as it rises 0.3 m, and a still blue ball, on a
 * checkered floor under a sphere light. Each moving thing has an
 * rtMotionBegin block at time 0 and one at time 1; cook interpolates them
 * at each sample's time, the turn along its arc. MotionBlurPan keys the
 * camera instead, a 0.5 m pan. 960 x 540, 256 spp, stratified, seed 0. */
static void MotionBlurScene(const char *name, bool pan)
{
    RtPoint eye = {0, 2.4f, 6.5f}, lookat = {0, 0.5f, 0};
    RtVector up = {0, 1, 0};
    RtColor ka = {0.04f, 0.04f, 0.045f};
    RtMaterial light = EMITTER(30, 30, 30);
    RtMaterial red = {{0.7f, 0.08f, 0.05f}, {0.3f, 0.3f, 0.3f}, 0.4f, 0, 0, 1.5f, {0, 0, 0}};
    RtMaterial yellow = PLASTIC(0.85f, 0.7f, 0.15f);
    RtMaterial blue = {{0.1f, 0.2f, 0.7f}, {0.3f, 0.3f, 0.3f}, 0.4f, 0, 0, 1.5f, {0, 0, 0}};
    CheckerParams checks = {16, 16, MATTE(0.75f, 0.75f, 0.72f), MATTE(0.2f, 0.2f, 0.22f)};

    rtBegin();
        rtFormat(960, 540);
        rtDisplay(name);
        if (pan) {
            /* the camera at two times: a 0.5 m pan to the right */
            RtPoint eye0 = {-0.25f, 2.4f, 6.5f}, eye1 = {0.25f, 2.4f, 6.5f};
            RtPoint at0 = {-0.25f, 0.5f, 0}, at1 = {0.25f, 0.5f, 0};
            rtMotionBegin(0);
                rtCamera(eye0, at0, up, 35);
            rtMotionEnd();
            rtMotionBegin(1);
                rtCamera(eye1, at1, up, 35);
            rtMotionEnd();
        } else {
            rtCamera(eye, lookat, up, 35);
        }
        rtPixelSamples(256);
        rtSampler("stratified");
        rtAmbient(ka);
        rtBackground(Color8(40, 44, 52));
        rtWorldBegin(0);
            rtMaterial(&light);
            rtTransformBegin(); rtTranslate(0, 4.5f, 1); rtSphere(0.5f); rtTransformEnd();

            rtMaterial(&checks.a);
            rtSurface(CHECKER, &checks, sizeof checks);
            Floor(8);
            rtSurface(NULL, NULL, 0);

            /* the red ball: 1.4 m to the right over the shutter */
            rtMaterial(&red);
            rtTransformBegin();
                rtMotionBegin(0);
                    rtTranslate(-2.4f, 0.4f, 0.4f);
                rtMotionEnd();
                rtMotionBegin(1);
                    rtTranslate(-1.0f, 0.4f, 0.4f);
                rtMotionEnd();
                rtSphere(0.4f);
            rtTransformEnd();

            /* the block: a 60 degree turn about its own axis and a 0.3 m
             * rise, both between the same two times (the turn is set
             * inside the block, so it interpolates along the arc) */
            rtMaterial(&yellow);
            rtTransformBegin();
                rtMotionBegin(0);
                    rtTranslate(0.6f, 0, -0.2f);
                    rtRotate(0, 0, 1, 0);
                rtMotionEnd();
                rtMotionBegin(1);
                    rtTranslate(0.6f, 0.3f, -0.2f);
                    rtRotate(60, 0, 1, 0);
                rtMotionEnd();
                Box(Vec3(-0.4f, 0, -0.4f), Vec3(0.4f, 0.8f, 0.4f));
            rtTransformEnd();

            /* the blue ball: no block, so no motion */
            rtMaterial(&blue);
            rtTransformBegin(); rtTranslate(2.3f, 0.4f, 0.6f); rtSphere(0.4f); rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}

void MotionBlur(void) { MotionBlurScene("4_ExtraCreditScenes/MotionBlur", false); }
void MotionBlurPan(void) { MotionBlurScene("4_ExtraCreditScenes/MotionBlurPan", true); }

/* 1984 ----------------------------------------------------------------- */

/* The pool table of Cook, Porter and Carpenter's "Distributed Ray
 * Tracing" (SIGGRAPH 1984), as rebuilt by Hachisuka et al. (SIGGRAPH
 * 2008, Figure 7): the table from straight above, the cue ball and the
 * 1, 9, 8 and 4 as the paper's picture has them, under one soft lamp
 * over the table's upper left: the 1 and the 4 at rest, the 9 streaking
 * across, the 8 spinning where it stands, the cue ball rolling in, each
 * moving ball keyed in its own rtMotionBegin block. Five spheres and a
 * quad of felt: under whitted the layout at the shutter's start, with a
 * hard shadow; under cook and kajiya motion blur, soft shadows and
 * glossy reflection in one picture. 1024 x 1024, 256 spp, halton, seed
 * 0. The model's ball and cloth maps, EXR converted to PNG; the 9 and
 * the 4 drawn in the style of its 1, 2, 5 and 8. */

/* The model's frame to the file's frame: (x, y, z) -> (x, z, -y), row form. */
static RtMatrix MODEL_TO_SCENE = {{1, 0, 0, 0}, {0, 0, -1, 0}, {0, 1, 0, 0}, {0, 0, 0, 1}};

/* BALL: a ball's map on a sphere, laid out as the model's mesh has it:
 * the map's columns run the other way round the ball than the sphere's
 * longitude, and its rows are not even in latitude (V_OF_POLAR, the
 * model's texture v every 5 degrees of the angle from the top; the
 * number sits where the rows crowd). */
typedef struct { RtInt tex; } BallParams;
static const float V_OF_POLAR[37] = {
    0.0058f, 0.0572f, 0.1153f, 0.1654f, 0.2086f, 0.2467f, 0.2811f, 0.3112f, 0.3366f, 0.3590f,
    0.3792f, 0.3977f, 0.4148f, 0.4302f, 0.4454f, 0.4596f, 0.4732f, 0.4873f, 0.5000f, 0.5127f,
    0.5268f, 0.5404f, 0.5546f, 0.5698f, 0.5852f, 0.6023f, 0.6208f, 0.6410f, 0.6634f, 0.6888f,
    0.7189f, 0.7533f, 0.7915f, 0.8346f, 0.8847f, 0.9428f, 0.9942f};
static void BALL(RtShadeContext *c)
{
    const BallParams *p = (const BallParams *)c->params;
    const float polar = 36 * std::fmin(std::fmax(c->v, 0.0f), 1.0f); /* in 5 degree steps */
    const int k = (int)std::fmin(polar, 35.0f);
    const float vModel = V_OF_POLAR[k] + (V_OF_POLAR[k + 1] - V_OF_POLAR[k]) * (polar - (float)k);
    RtFloat rgba[4];
    rtTexture(p->tex, 1 - c->u, 1 - vModel, rgba); /* the model's v counts up from the bottom row */
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = rgba[i];
    }
}

/* FELT: the cloth's map, tiled six times across the table as the model
 * has it. */
typedef struct { RtInt tex; } FeltParams;
static void FELT(RtShadeContext *c)
{
    const FeltParams *p = (const FeltParams *)c->params;
    const RtFloat s = 6 * c->u, t = 6 * c->v;
    RtFloat rgba[4];
    rtTexture(p->tex, s - std::floor(s), t - std::floor(t), rgba);
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = rgba[i];
    }
}

/* One key of a ball's motion: time, position, turn (degrees about an
 * axis), from pool.env. */
struct BallKey {
    float t;
    float x, y, z;
    float angle, ax, ay, az;
};

/* A ball: a sphere of radius 100 in its map (-1: the cue ball's plain
 * color) and its keys. */
static void Ball(const RtMaterial *m, RtInt tex, const BallKey *keys, int nkeys)
{
    rtMaterial(m);
    if (tex >= 0) {
        BallParams bp = {tex};
        rtSurface(BALL, &bp, sizeof bp);
    } else {
        rtSurface(NULL, NULL, 0);
    }
    rtTransformBegin();
        for (int i = 0; i < nkeys; i++) {
            const BallKey &k = keys[i];
            if (nkeys > 1) {
                rtMotionBegin(k.t);
            }
            rtTranslate(k.x, k.y, k.z);
            rtRotate(k.angle, k.ax, k.ay, k.az);
            if (nkeys > 1) {
                rtMotionEnd();
            }
        }
        rtConcatTransform(MODEL_TO_SCENE); /* the model's turn, before the motion */
        rtSphere(100);
    rtTransformEnd();
}

void Pool1984(void) /* the scene is named 1984; a C name cannot start with a digit */
{
    /* The file's units are millimeters or so (a ball is 200 across, the
     * camera 3750 up), far too large for the renderer's epsilon, which is
     * set for meters: everything below is drawn under a scale of 0.001. */
    const float S = 0.001f;

    /* pool.env's camera: 3.75 m straight above the table's middle (the
     * file's z is depth, toward the table), looking down, 14 degrees. */
    RtPoint eye = {-50 * S, -50 * S, -3750 * S}, lookat = {-50 * S, -50 * S, 0};
    RtVector up = {0, -1, 0}; /* the balls' numbers read upright this way, as the paper's picture has them */

    /* The lamp: a quad 11 x 7 of the file's units over the table's upper
     * left, facing it, the picture's one soft light; about 7 W/m^2 on the
     * cloth, the shadows falling down and to the right. */
    RtMaterial lamp = EMITTER(48, 47, 43);

    /* Materials: the balls' glTF has metallic 0.2 and roughness 0.3, a
     * glossy phenolic: specular 0.1 (the reflectance head on) at
     * roughness 0.35, so the room shows in them only as soft highlights,
     * as on a pool ball, not as in a mirror; the cloth is its felt map,
     * nearly matte. */
    RtMaterial ball = {{0.8f, 0.8f, 0.8f}, {0.1f, 0.1f, 0.1f}, 0.35f, 0, 0, 1.5f, {0, 0, 0}};
    RtMaterial cue = {{0.92f, 0.9f, 0.72f}, {0.1f, 0.1f, 0.1f}, 0.35f, 0, 0, 1.5f, {0, 0, 0}};
    RtMaterial cloth = {{0.3f, 1.0f, 0.45f}, {0.02f, 0.02f, 0.02f}, 0.7f, 0, 0, 1.5f, {0, 0, 0}};
    RtColor ka = {0.06f, 0.07f, 0.06f};

    /* The keys, from pool.env: time, position, turn. */
    const BallKey cueKeys[] = {{0.0f, 0, 320, 0, 0, 0, 1, 0}, {1.0f, -6, 275, 0, 20, 1, 0, 0}};
    const BallKey fourKeys[] = {{0.0f, 290, -150, 0, 102, 1.8f, -0.37f, 0.62f}};
    const BallKey eightKeys[] = {{0.0f, 50, -20, 0, 82, 1.97f, -0.1f, -0.1f},
                                {1.0f, 60, -10, 0, 232, 1.97f, -0.1f, -0.1f}}; /* spinning where it stands */
    const BallKey nineKeys[] = {{0.0f, -150, -90, 0, 90, 1.97f, -0.093f, -0.28f},
                                 {1.0f, -205, 5, 0, 150, 1.96f, 0.2f, -0.2f}};
    const BallKey oneKeys[] = {{0.0f, -350, -170, 0, 105, 1.8f, -0.35f, 0.53f}};

    rtBegin();
        rtFormat(1024, 1024);
        rtDisplay("4_ExtraCreditScenes/1984");
        rtTonemap("filmic", 0.0f);
        rtCamera(eye, lookat, up, 14);
        rtPixelSamples(256);
        rtSampler("halton");
        rtAmbient(ka);
        rtBackground(Color8(14, 14, 14)); /* the dark room the balls reflect */
        rtWorldBegin(0);
          rtTransformBegin();
            rtScale(S, S, S); /* the whole world: the file's units to meters */
            rtMaterial(&lamp);
            Quad(Vec3(-1500, -1650, -1600), Vec3(0, 700, 0), Vec3(1100, 0, 0)); /* faces +z, toward the table */

            const RtInt tex1 = rtMakeTexture("models/pool/textures/ball_1.png", 1);
            const RtInt tex4 = rtMakeTexture("models/pool/textures/ball_4.png", 1);
            const RtInt tex8 = rtMakeTexture("models/pool/textures/ball_8.png", 1);
            const RtInt tex9 = rtMakeTexture("models/pool/textures/ball_9.png", 1);
            const RtInt texCloth = rtMakeTexture("models/pool/textures/table.png", 1);

            /* The cloth: the model's playing surface, a quad at the balls'
             * feet (they rest at z = 100), in its felt. */
            rtMaterial(&cloth);
            FeltParams fp = {texCloth};
            rtSurface(FELT, &fp, sizeof fp);
            Quad(Vec3(-1642, -1532, 101), Vec3(3261, 0, 0), Vec3(0, 3060, 0)); /* faces -z, up at the camera */

            Ball(&cue, -1, cueKeys, 2);
            Ball(&ball, tex4, fourKeys, 1);
            Ball(&ball, tex8, eightKeys, 2);
            Ball(&ball, tex9, nineKeys, 2);
            Ball(&ball, tex1, oneKeys, 1);
            rtSurface(NULL, NULL, 0);
          rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}

/* MoleculeFastMovie ---------------------------------------------------- */

/* HW2's molecule (scenes/1_Whitted/1_TestScenes/Molecule.cpp) turning 10
 * times over its 96 frames, 37.5 degrees a frame, each frame exposed over
 * its whole step so the trails join: whitted strobes, cook's blur reads
 * as a spin. The speed in turns is the scene's argument (gauchoRT
 * MoleculeFastMovie 6). 720 x 720, 64 spp, halton, seed 0. */
void MoleculeScene(int frame, const char *folder, const char *movie, float turns, float shutter);

void MoleculeFastMovie(void) { MoleculeScene(-1, "4_ExtraCreditScenes", "MoleculeFastMovie", 10, 1); }
