/*
 * ECE/CS 285 – HW2 Benchmark Scene: Forest
 * Sixty-four trees in a loose grid on a rolling terrain built from
 * perlinNoise (256 x 256, 131,072 triangles with vertex normals) under a
 * morning sun and a hazy sky: 7.4 million triangles, one 113,022-triangle
 * tree drawn 64 times with 46,000 alpha-cutout leaf quads; memory, build
 * time, and cutout tests on every shadow ray through the canopy; 256 spp,
 * halton, seed 0 under cook and kajiya.
 * Model: the Barcelona Pavilion's tree (eMirage, CC-BY),
 * models/tree/tree.mesh.
 *
 * Output: renders/<integrator>/2_BenchmarkScenes/Forest.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

#include <cmath>
#include <vector>

/* The ground height at (x, z): two octave sums of the renderer's noise,
 * low rolling hills and finer bumps. */
static float Height(float x, float z)
{
    return 5.0f * (float)Fbm(x / 45.0, z / 45.0, 0.37, 4) + 1.2f * (float)Fbm(x / 9.0 + 7.1, z / 9.0 + 3.3, 1.9, 3);
}

/* A repeatable pseudo-random number in [0, 1). */
static float Random(unsigned &state)
{
    state = state * 1664525u + 1013904223u;
    return (float)(state >> 8) / 16777216.0f;
}

void Forest(void)
{
    RtPoint eye = {0, 12, 78}, lookat = {0, 5, 0};
    RtVector up = {0, 1, 0};
    RtVector sunDir = {-0.45f, -1, -0.35f};
    RtColor sunE = {3.4f, 3.2f, 2.8f};
    RtColor ka = {0.16f, 0.17f, 0.19f};
    RtMaterial ground = MATTE(0.22f, 0.42f, 0.16f);

    rtBegin();
        rtFormat(1920, 1080);
        rtDisplay("2_BenchmarkScenes/Forest");
        rtPixelSamples(256); /* cook and kajiya; whitted renders one sample */
        rtSampler("halton");
        rtCamera(eye, lookat, up, 48);
        rtAmbient(ka);
        rtBackground(Color8(214, 226, 232)); /* morning haze */
        rtWorldBegin(0);
            rtDirectionalLight(sunDir, sunE);

            /* The terrain: 200 m across, with normals from the height
             * function's slopes. */
            const int n = 256;
            const float size = 200;
            std::vector<RtFloat> P((size_t)(n + 1) * (n + 1) * 3), N((size_t)(n + 1) * (n + 1) * 3);
            std::vector<RtInt> idx;
            for (int j = 0; j <= n; j++)
                for (int i = 0; i <= n; i++) {
                    const float x = size * ((float)i / n - 0.5f), z = size * ((float)j / n - 0.5f);
                    const float h = Height(x, z), d = 0.5f;
                    const float dx = (Height(x + d, z) - Height(x - d, z)) / (2 * d);
                    const float dz = (Height(x, z + d) - Height(x, z - d)) / (2 * d);
                    const Vec3 normal = normalize(Vec3(-dx, 1, -dz));
                    RtFloat *p = &P[3 * ((size_t)j * (n + 1) + i)];
                    p[0] = x;
                    p[1] = h;
                    p[2] = z;
                    normal.store(&N[3 * ((size_t)j * (n + 1) + i)]);
                }
            for (int j = 0; j < n; j++)
                for (int i = 0; i < n; i++) {
                    const int a = j * (n + 1) + i, b = a + 1, c = a + n + 1, d = c + 1;
                    idx.insert(idx.end(), {a, c, b, b, c, d});
                }
            rtMaterial(&ground);
            rtTriangleMesh((RtInt)(P.size() / 3), (const RtPoint *)P.data(), (const RtNormal *)N.data(), NULL, NULL,
                           (RtInt)(idx.size() / 3), idx.data());

            /* The trees: an 8 x 8 grid 13 m apart, each jittered, turned
             * and scaled by its own random numbers, its trunk base (the
             * model's origin, 11.5 m tall at scale 1) on the ground. */
            ObjMesh *tree = obj_load("models/tree/tree.mesh");
            unsigned seed = 12345u;
            for (int j = 0; j < 8; j++)
                for (int i = 0; i < 8; i++) {
                    const float x = 13.0f * (i - 3.5f) + 6 * (Random(seed) - 0.5f);
                    const float z = 13.0f * (j - 3.5f) + 6 * (Random(seed) - 0.5f);
                    const float scale = 0.6f + 0.6f * Random(seed); /* 7 to 14 m tall */
                    const float turn = 360 * Random(seed);
                    rtTransformBegin();
                        rtTranslate(x, Height(x, z) - 0.1f, z);
                        rtRotate(turn, 0, 1, 0);
                        rtScale(scale, scale, scale);
                        ObjDraw(tree);
                    rtTransformEnd();
                }
            obj_free(tree);
        rtWorldEnd();
    rtEnd();
}
