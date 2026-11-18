/*
 * ECE/CS 285 – HW3 Test Scene: WarpView
 * A warp's samples as small red spheres on a white copy of its domain, so
 * a clump, a seam or a hole shows before the chi-square test reports it.
 * It draws the dumps of your own --warptest that
 * python tools/warptest.py all --view writes to warptest/, one image per
 * dump. A direction is a point on the unit sphere, seen twice: from the
 * side, tilted toward you, and down its axis (the blue dot is +z, the dark
 * ring the equator). The disk and the triangle are seen head on.
 * 960 x 540, 16 spp, stratified, seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/WarpView_<warp>.png
 */

#include "rt.h"
#include "sceneutil.h"

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <string>
#include <vector>

enum WarpDomain { ON_DISK, ON_SPHERE, ON_TRIANGLE };

struct WarpEntry {
    const char *name;
    WarpDomain domain;
};

/* A warp's place here is its frame number, which tools/warptest.py asks
 * for with --frames: the two lists keep the same order. */
static const WarpEntry WARP_ENTRIES[] = {{"disk", ON_DISK},          {"cosine_hemisphere", ON_SPHERE},
                                         {"uniform_sphere", ON_SPHERE}, {"uniform_cone", ON_SPHERE},
                                         {"triangle", ON_TRIANGLE},  {"phong", ON_SPHERE},
                                         {"ggx", ON_SPHERE}};

/* The first three numbers of each line of warptest/<warp>.txt, flat; empty
 * when there is no such file. strtof reads a nan or an inf that a broken
 * pdf printed, where >> would stop at it. */
static std::vector<float> readDump(const char *warp)
{
    std::vector<float> points;
    std::ifstream file(std::string("warptest/") + warp + ".txt");
    std::string line;
    while (std::getline(file, line)) {
        const char *s = line.c_str();
        float v[3];
        bool ok = true;
        for (int k = 0; k < 3; k++) {
            char *end;
            v[k] = std::strtof(s, &end);
            ok = ok && end != s && std::isfinite(v[k]);
            s = end;
        }
        if (ok) {
            points.insert(points.end(), v, v + 3);
        }
    }
    return points;
}

static void drawPoints(const std::vector<float> &points, float radius)
{
    RtMaterial red = MATTE(0.8f, 0.02f, 0.02f);
    rtMaterial(&red);
    for (size_t i = 0; i + 2 < points.size(); i += 3) {
        rtTransformBegin();
            rtTranslate(points[i], points[i + 1], points[i + 2]);
            rtSphere(radius);
        rtTransformEnd();
    }
}

/* The unit sphere with its equator and +z marked, and the directions on it. */
static void drawSphereDomain(const std::vector<float> &points, float radius)
{
    RtMaterial white = MATTE(0.8f, 0.8f, 0.8f);
    RtMaterial ink = MATTE(0.02f, 0.02f, 0.02f);
    RtMaterial blue = MATTE(0.05f, 0.2f, 0.9f);
    rtMaterial(&white);
    rtSphere(1);
    rtMaterial(&ink);
    for (int i = 0; i < 180; i++) {
        const float a = 2 * 3.14159265f * (float)i / 180.0f;
        rtTransformBegin();
            rtTranslate(std::cos(a), std::sin(a), 0);
            rtSphere(0.006f);
        rtTransformEnd();
    }
    rtMaterial(&blue);
    rtTransformBegin();
        rtTranslate(0, 0, 1);
        rtSphere(0.03f);
    rtTransformEnd();
    drawPoints(points, radius);
}

/* The domain and its points, sized so the spheres cover about the same
 * share of what the points occupy at any count: for directions, the cap
 * above the lowest tenth of their z, so a narrow lobe's stay apart. */
static void drawWarp(WarpDomain domain, const std::vector<float> &dump)
{
    RtMaterial white = MATTE(0.8f, 0.8f, 0.8f);
    const float root = std::sqrt((float)(dump.size() / 3));
    if (domain == ON_SPHERE) {
        std::vector<float> z;
        for (size_t i = 2; i < dump.size(); i += 3) {
            z.push_back(dump[i]);
        }
        std::nth_element(z.begin(), z.begin() + (std::ptrdiff_t)(z.size() / 10), z.end());
        const float cap = 2 * 3.14159265f * (1 - z[z.size() / 10]);
        const float radius = std::max(0.008f, std::min(0.05f, 0.22f * std::sqrt(cap) / root));
        rtTransformBegin(); /* from the side: +z up, tilted 20 degrees toward the camera */
            rtTranslate(-1.1f, 0, 0);
            rtRotate(-70, 1, 0, 0);
            drawSphereDomain(dump, radius);
        rtTransformEnd();
        rtTransformBegin(); /* down the axis: +z at the camera, +x right, +y up */
            rtTranslate(1.1f, 0, 0);
            drawSphereDomain(dump, radius);
        rtTransformEnd();
    } else if (domain == ON_DISK) {
        rtTransformBegin();
            rtScale(1.1f, 1.1f, 1.1f);
            rtMaterial(&white);
            Disc(Vec3(0, 0, 0), Vec3(1, 0, 0), Vec3(0, 1, 0), 1, 128);
            drawPoints(dump, std::min(0.05f, 0.45f / root));
        rtTransformEnd();
    } else {
        /* Equilateral: b0 at the top, b1 lower left, b2 lower right. */
        const float A[3] = {0, 1, 0}, B[3] = {-0.8660254f, -0.5f, 0}, C[3] = {0.8660254f, -0.5f, 0};
        const float corners[9] = {A[0], A[1], A[2], B[0], B[1], B[2], C[0], C[1], C[2]};
        std::vector<float> points(dump.size());
        for (size_t i = 0; i + 2 < dump.size(); i += 3) {
            for (size_t k = 0; k < 3; k++) {
                points[i + k] = dump[i] * A[k] + dump[i + 1] * B[k] + dump[i + 2] * C[k];
            }
        }
        rtTransformBegin();
            rtTranslate(0, -0.3f, 0);
            rtScale(1.25f, 1.25f, 1.25f);
            rtMaterial(&white);
            Triangles((const RtPoint *)corners, 1);
            drawPoints(points, std::min(0.05f, 0.4f / root));
        rtTransformEnd();
    }
}

void WarpView(void)
{
    const int count = (int)(sizeof WARP_ENTRIES / sizeof WARP_ENTRIES[0]);
    std::vector<std::vector<float>> dumps;
    int found = 0;
    for (int i = 0; i < count; i++) {
        dumps.push_back(readDump(WARP_ENTRIES[i].name));
        found += dumps.back().empty() ? 0 : 1;
    }
    if (found == 0) {
        std::fprintf(stderr, "WarpView: no dumps in warptest/, so no image; run python tools/warptest.py all --view\n");
        return;
    }

    RtPoint eye = {0, 0, 7.1f}, lookat = {0, 0, 0};
    RtVector up = {0, 1, 0};
    RtVector lightDirection = {-0.25f, -0.35f, -1};
    RtColor lightE = {2.2f, 2.2f, 2.2f};
    RtColor ka = {0.35f, 0.35f, 0.35f};
    RtColor backdrop = {0.06f, 0.06f, 0.06f};

    rtBegin();
        rtFormat(960, 540);
        rtCamera(eye, lookat, up, 20);
        rtPixelSamples(16);
        rtSampler("stratified");
        rtAmbient(ka);
        rtBackground(backdrop);
        for (int i = 0; i < count; i++) {
            if (dumps[(size_t)i].empty()) {
                continue;
            }
            char name[96];
            std::snprintf(name, sizeof name, "1_TestScenes/WarpView_%s", WARP_ENTRIES[i].name);
            rtDisplay(name);
            if (rtWorldBegin(i)) {
                rtDirectionalLight(lightDirection, lightE);
                drawWarp(WARP_ENTRIES[i].domain, dumps[(size_t)i]);
                rtWorldEnd();
            }
        }
    rtEnd();
}
