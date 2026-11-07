/*
 * ECE/CS 285 – HW2 Hero Scene: Chess
 * The chess scene of Hachisuka et al., "Multidimensional Adaptive
 * Sampling and Reconstruction for Ray Tracing" (SIGGRAPH 2008, Figure 8):
 * a 40 cm board of two marbles, a black queen and pawn in worn lacquer
 * (the scene's own shader: mottled, bumped, its gloss wandering) before
 * ivory pieces, 25 cm from the queen through a 24 degree lens 6 mm in
 * radius focused on her, under a soft key, a cool rim and a fill, all
 * sphere lamps: depth of field and soft shadows at once. 60,000
 * triangles. 1080 x 1440, 1024 spp, halton, seed 0.
 * Model: the pmdas repository's chess scene (github.com/meistdan/pmdas,
 * SDK/data/chess), modeled by Wojciech Jarosz, for non-commercial use
 * with this credit; EXR textures converted to PNG; chess.env's z-down
 * frame read as (x, -z, y); its point lights made 3 cm spheres.
 * gauchoRT Chess 1 renders the board from straight above, sharp, a
 * view for seeing which lamp and piece throw a shadow.
 *
 * Output: renders/<integrator>/3_HeroScenes/Chess.png (Chess_above.png)
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"
#include "shaders.h"

#include <cmath>

/* LACQUER: the pieces' worn finish. The albedo is the base color mottled
 * by a noise (scale meters, the mottle's amplitude and color), the
 * roughness wanders with a second noise between its two bounds, and a
 * third, fine noise bumps the normal (its gradient by central differences,
 * scaled by `bump`), so the highlights break up as on a handled piece. */
typedef struct {
    RtColor base, mottle;
    RtFloat mottleScale, mottleAmount;
    RtFloat roughLow, roughHigh, roughScale;
    RtFloat bump, bumpScale;
} LacquerParams;

static void LACQUER(RtShadeContext *c)
{
    const LacquerParams *p = (const LacquerParams *)c->params;
    const double x = c->Po[0], y = c->Po[1], z = c->Po[2];
    const double m = 0.5 + 0.5 * Fbm(x / p->mottleScale, y / p->mottleScale, z / p->mottleScale, 4);
    const float t = (float)std::pow(std::min(1.0, std::max(0.0, m)), 2.0) * p->mottleAmount;
    for (int i = 0; i < 3; i++)
        c->m.albedo[i] = p->base[i] + t * (p->mottle[i] - p->base[i]);
    const double r = 0.5 + 0.5 * perlinNoise(x / p->roughScale, y / p->roughScale, z / p->roughScale);
    c->m.roughness = p->roughLow + (float)r * (p->roughHigh - p->roughLow);
    if (p->bump > 0) {
        const double h = p->bumpScale * 0.25, sc = p->bumpScale;
        const double gx = (Fbm((x + h) / sc, y / sc, z / sc, 3) - Fbm((x - h) / sc, y / sc, z / sc, 3)) / (2 * h);
        const double gy = (Fbm(x / sc, (y + h) / sc, z / sc, 3) - Fbm(x / sc, (y - h) / sc, z / sc, 3)) / (2 * h);
        const double gz = (Fbm(x / sc, y / sc, (z + h) / sc, 3) - Fbm(x / sc, y / sc, (z - h) / sc, 3)) / (2 * h);
        const Vec3 N(c->N), g((float)gx, (float)gy, (float)gz);
        const Vec3 gt = g - N * dot(g, N); /* the gradient's part along the surface */
        const Vec3 n = normalize(N - gt * (p->bump * p->bumpScale));
        if (dot(n, Vec3(c->Ng)) > 0)
            n.store(c->N);
    }
}

/* One piece: its OBJ, drawn with the material bound here (the files
 * carry none) and the lacquer (or, for the board's squares, a marble),
 * moved by (dx, dz) on the board for the two the file places by a node
 * translation. */
static void Piece(const char *file, const RtMaterial *m, const MarbleParams *marble, const LacquerParams *lacquer, float dx, float dz)
{
    ObjMesh *mesh = obj_load(file);
    rtMaterial(m);
    if (marble) {
        rtSurface(MARBLE, marble, sizeof *marble);
    } else if (lacquer) {
        rtSurface(LACQUER, lacquer, sizeof *lacquer);
    } else {
        rtSurface(NULL, NULL, 0);
    }
    rtTransformBegin();
        rtTranslate(dx, 0, dz);
        ObjDraw(mesh, OBJ_KEEP_MATERIAL);
    rtTransformEnd();
    obj_free(mesh);
}

void Chess(void)
{
    /* The camera: chess.env's view, brought in to 25 cm from the black
     * queen at her own height, tilted 6.7 degrees down so her base and
     * the king's cross both sit in the frame, a 24 degree lens, a 6 mm
     * aperture focused on her front: the pieces behind dissolve. From
     * above (the scene's argument 1): 90 cm over the board's center,
     * -z up in the picture, so this camera sits at the bottom right. */
    const bool fromAbove = rtSceneArg(0, 0) > 0;
    RtPoint eye = {0.2081f - 0.1211f * 0.55f, 0.075f, 0.5019f - 0.2579f * 0.55f};
    RtPoint lookat = {0.2081f - 0.1211f * 1.1f, 0.0565f, 0.5019f - 0.2579f * 1.1f};
    RtVector up = {0, 1, 0};
    RtPoint aboveEye = {0, 0.9f, 0}, aboveLookat = {0, 0, 0};
    RtVector aboveUp = {0, 0, -1};

    /* The lamps: a soft key, a 10 cm sphere high on the camera's left,
     * that lights the bishop's near side and throws the long soft shadows;
     * a smaller cool rim high on the far right that draws the pieces off
     * the black; a dim fill above the camera. Radiance I / (pi r^2). */
    const float lamps[3][5] = {{-0.20f, 0.55f, 0.60f, 0.09f, 0.9f},  /* x, y, z, radius, I */
                               {0.45f, 0.40f, -0.30f, 0.07f, 0.09f},
                               {0.20f, 0.35f, 0.70f, 0.10f, 0.045f}};

    /* The materials: the white pieces' gltf has base color 0.9 and
     * roughness 0.5, a satin plastic (specular 0.35 at roughness 0.45: a
     * wide, soft highlight, no mirror); the black pieces a dark lacquer
     * (specular 0.25 at roughness 0.3: a faint mirror); the board's sides
     * grey 0.4. */
    RtMaterial white = {{0.9f, 0.87f, 0.8f}, {0.35f, 0.35f, 0.35f}, 0.45f, 0, 0, 1.5f, {0, 0, 0}};
    RtMaterial black = {{0.03f, 0.03f, 0.032f}, {0.25f, 0.25f, 0.25f}, 0.3f, 0, 0, 1.5f, {0, 0, 0}};
    /* The finishes (LACQUER): the black pieces' lacquer worn to grey in
     * patches, its gloss wandering, and bumped by a fine noise that breaks
     * the highlights up as on a handled piece; the white ones' ivory,
     * satin, with a faint grain and lighter bumps. */
    const LacquerParams blackLacquer = {{0.03f, 0.03f, 0.032f}, {0.17f, 0.17f, 0.18f}, 0.012f, 1.0f, 0.3f, 0.45f, 0.004f, 0.12f, 0.0035f};
    const LacquerParams whiteLacquer = {{0.9f, 0.87f, 0.8f}, {0.7f, 0.66f, 0.58f}, 0.02f, 0.4f, 0.4f, 0.55f, 0.006f, 0.12f, 0.008f};
    /* The board: squares of two marbles (MARBLE, sceneutil.h) under a
     * polished finish that shows the pieces in it, blurred, the veins
     * carved into the board's own space; the two noises differ in scale
     * so the light and dark slabs do not share a vein. */
    RtMaterial squares = {{0.6f, 0.6f, 0.6f}, {0.25f, 0.25f, 0.25f}, 0.22f, 0, 0, 1.5f, {0, 0, 0}};
    const MarbleParams whiteMarble = {0.026f, 0.7f, 0.05f, 0.15f, {0.82f, 0.8f, 0.76f}, {0.3f, 0.3f, 0.35f}};
    const MarbleParams darkMarble = {0.031f, 0.8f, 0.06f, 0.3f, {0.025f, 0.025f, 0.03f}, {0.26f, 0.26f, 0.3f}};
    RtMaterial sides = {{0.4f, 0.4f, 0.4f}, {0.06f, 0.06f, 0.06f}, 0.6f, 0, 0, 1.5f, {0, 0, 0}};
    RtColor ka = {0.004f, 0.004f, 0.004f};
    RtColor blackBackground = {0, 0, 0};

    rtBegin();
        if (fromAbove) {
            rtFormat(1080, 1080);
            rtDisplay("3_HeroScenes/Chess_above");
            rtCamera(aboveEye, aboveLookat, aboveUp, 30);
        } else {
            rtFormat(1080, 1440);
            rtDisplay("3_HeroScenes/Chess");
            rtCamera(eye, lookat, up, 24);
            rtLens(0.006f, 0.251f);
        }
        rtTonemap("filmic", 0.7f);
        rtPixelSamples(1024);
        rtSampler("halton");
        rtAmbient(ka);
        rtBackground(blackBackground);
        rtWorldBegin(0);
            for (int i = 0; i < 3; i++) {
                const float r = lamps[i][3], Le = lamps[i][4] / (3.14159265f * r * r);
                RtMaterial lamp = EMITTER(Le, Le, Le * (i == 1 ? 1.15f : 0.95f));
                rtMaterial(&lamp);
                rtTransformBegin();
                    rtTranslate(lamps[i][0], lamps[i][1], lamps[i][2]);
                    rtSphere(r);
                rtTransformEnd();
            }
            Piece("models/chessmdas/white_squares2.mesh", &squares, &whiteMarble, NULL, 0, 0);
            Piece("models/chessmdas/black_squares2.mesh", &squares, &darkMarble, NULL, 0, 0);
            Piece("models/chessmdas/chessboard.sides.mesh", &sides, NULL, NULL, 0, 0);
            Piece("models/chessmdas/bishop1.white.mesh", &white, NULL, &whiteLacquer, 0, 0);
            Piece("models/chessmdas/bishop2.white.mesh", &white, NULL, &whiteLacquer, 0, 0);
            Piece("models/chessmdas/king.white.mesh", &white, NULL, &whiteLacquer, 0, 0);
            Piece("models/chessmdas/pawn1.white.mesh", &white, NULL, &whiteLacquer, 0, 0);
            Piece("models/chessmdas/pawn2.white.mesh", &white, NULL, &whiteLacquer, 0, 0);
            Piece("models/chessmdas/queen.white.mesh", &white, NULL, &whiteLacquer, 0, 0);
            Piece("models/chessmdas/black_pawn3.mesh", &black, NULL, &blackLacquer, 0.0343f, 0.0827f);
            Piece("models/chessmdas/black_queen3.mesh", &black, NULL, &blackLacquer, 0.0237f, 0.1234f);
            rtSurface(NULL, NULL, 0);
        rtWorldEnd();
    rtEnd();
}
