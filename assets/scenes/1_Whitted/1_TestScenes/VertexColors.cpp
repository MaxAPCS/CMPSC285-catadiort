/*
 * ECE/CS 285 – HW2 Test Scene: VertexColors
 * The Stanford Happy Buddha, 293,232 triangles, painted by a color per
 * vertex that the scene computes (rtTriangleMesh's Cs): red at the feet
 * to gold at the head, with teal veins where a noise crosses zero, the
 * one test of Cs and its interpolation across a triangle; a mirror ball
 * beside it reflects the mesh. Three point lights. 720 x 720; 256 spp,
 * halton, seed 0 under cook and kajiya. Model: Happy Buddha, Stanford 3D
 * Scanning Repository (for education).
 *
 * Output: renders/<integrator>/1_TestScenes/VertexColors.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

#include <algorithm>
#include <cmath>
#include <vector>

/* The color of a vertex at P (model space, meters, the statue 0.05 to
 * 0.25 up): the gradient by height, the veins by a noise about 4 cm
 * across. */
static void BuddhaColor(const RtPoint P, RtFloat c[3])
{
    const float t = std::max(0.0f, std::min(1.0f, (P[1] - 0.05f) / 0.2f));
    const float s = t * t * (3 - 2 * t);
    const float red[3] = {0.50f, 0.06f, 0.04f}, gold[3] = {0.95f, 0.72f, 0.25f}, teal[3] = {0.08f, 0.45f, 0.50f};
    const double n = perlinNoise(P[0] * 25.0 + 3.1, P[1] * 25.0, P[2] * 25.0 - 1.7);
    const float v = std::max(0.0f, std::min(1.0f, ((float)std::fabs(n) - 0.015f) / 0.06f)); /* 0 on a vein */
    const float vein = v * v * (3 - 2 * v);
    for (int i = 0; i < 3; i++) {
        const float base = red[i] + (gold[i] - red[i]) * s;
        c[i] = teal[i] + (base - teal[i]) * vein;
    }
}

void VertexColors(void)
{
    RtPoint eye = {0.85f, 0.72f, 2.0f}, lookat = {0.0f, 0.46f, 0.0f};
    RtVector up = {0, 1, 0};
    RtColor ka = {0.03f, 0.03f, 0.035f};
    RtColor background = {0.08f, 0.09f, 0.11f};
    RtPoint keyPos = {-1.6f, 2.8f, 2.2f}, fillPos = {2.5f, 1.2f, 1.8f}, rimPos = {1.5f, 2.5f, -2.5f};
    RtColor keyI = {46, 44, 40}, fillI = {8, 9, 11}, rimI = {26, 26, 28};
    RtMaterial statue = {{0.8f, 0.8f, 0.8f}, {0.25f, 0.25f, 0.25f}, 0.4f, 0, 0, 1.5f, {0, 0, 0}}; /* the albedo is Cs */
    RtMaterial floor = MATTE(0.18f, 0.16f, 0.15f);
    RtMaterial ball = MIRROR;

    rtBegin();
        rtFormat(720, 720);
        rtDisplay("1_TestScenes/VertexColors");
        rtPixelSamples(256);
        rtSampler("halton");
        rtCamera(eye, lookat, up, 34.0f);
        rtAmbient(ka);
        rtBackground(background);
        rtWorldBegin(0);
            rtPointLight(keyPos, keyI);
            rtPointLight(fillPos, fillI);
            rtPointLight(rimPos, rimI);

            rtMaterial(&floor);
            Quad(Vec3(-6, 0, -6), Vec3(12, 0, 0), Vec3(0, 0, 12));

            rtMaterial(&ball);
            rtTransformBegin();
                rtTranslate(0.58f, 0.2f, 0.42f);
                rtSphere(0.2f);
            rtTransformEnd();

            /* The statue, 1 m tall on the floor; its colors from BuddhaColor
             * on the model's own coordinates. */
            rtMaterial(&statue);
            ObjMesh *buddha = obj_load("models/buddha/buddha.mesh");
            if (buddha) {
                rtTransformBegin();
                    rtTranslate(0.0f, -0.25f, 0.0f);
                    rtScale(5, 5, 5);
                    rtTranslate(0.005f, 0.0f, 0.007f); /* its bounds' center */
                    std::vector<RtInt> indices;
                    std::vector<RtFloat> Cs; /* 3 per vertex */
                    for (int k = 0; k < buddha->ngroups; k++) {
                        const ObjGroup &g = buddha->groups[k];
                        Cs.resize(3 * (size_t)g.nvertices);
                        for (RtInt i = 0; i < g.nvertices; i++)
                            BuddhaColor(g.P[i], &Cs[3 * (size_t)i]);
                        indices.clear();
                        const RtInt *corner = g.verts;
                        for (RtInt p = 0; p < g.npolys; p++) { /* fanned, as ObjDraw does */
                            for (RtInt i = 1; i + 1 < g.nverts[p]; i++) {
                                indices.push_back(corner[0]);
                                indices.push_back(corner[i]);
                                indices.push_back(corner[i + 1]);
                            }
                            corner += g.nverts[p];
                        }
                        rtTriangleMesh(g.nvertices, g.P, g.N, NULL, reinterpret_cast<const RtColor *>(Cs.data()),
                                       (RtInt)(indices.size() / 3), indices.data());
                    }
                rtTransformEnd();
                obj_free(buddha);
            }
        rtWorldEnd();
    rtEnd();
}
