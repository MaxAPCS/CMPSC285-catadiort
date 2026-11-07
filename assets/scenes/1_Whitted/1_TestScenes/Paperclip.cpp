/*
 * ECE/CS 285 – HW2 Test Scene: Paperclip
 * After Cook, Porter and Carpenter's "Distributed Ray Tracing" (SIGGRAPH
 * 1984): a steel paper clip standing on a varnished oak table under one
 * small lamp, before a dark wall. The clip's reflection blurs as it
 * stretches across the table and its shadow softens with the lamp's size:
 * glossy reflection and soft shadows in one picture. The clip is a tube
 * swept along its wire, built here; the oak is WOOD from sceneutil.h.
 * 960 x 720, 512 spp (enough: at 2048 the glossy caustic under the clip
 * differs by under half an 8-bit level), halton, seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/Paperclip.png
 */

#include "rt.h"
#include "sceneutil.h"

#include <cmath>
#include <vector>

/* The wire's path: a Gem clip 34 mm long and 8 mm wide in the x-y plane,
 * one polyline from its outer free end to its inner one, the arcs
 * sampled every 10 degrees. Millimeters. */
static void ClipPath(std::vector<Vec3> &path)
{
    const float pi = 3.14159265f;
    /* a straight run from a to b, then an arc about c of radius r from
     * angle a0 to a1 (degrees), each appending its end point */
    path.push_back(Vec3(4, 4, 0));
    path.push_back(Vec3(4, 28, 0));                           /* outer right leg, up */
    for (int i = 10; i <= 180; i += 10) {                     /* outer top arc */
        const float a = i * pi / 180;
        path.push_back(Vec3(4 * std::cos(a), 28 + 4 * std::sin(a), 0));
    }
    path.push_back(Vec3(-4, 4, 0));                           /* outer left leg, down */
    for (int i = 10; i <= 180; i += 10) {                     /* bottom arc, tightening */
        const float a = pi + i * pi / 180;
        path.push_back(Vec3(-1 + 3 * std::cos(a), 4 + 3 * std::sin(a), 0));
    }
    path.push_back(Vec3(2, 22, 0));                           /* inner right leg, up */
    for (int i = 10; i <= 180; i += 10) {                     /* inner top arc */
        const float a = i * pi / 180;
        path.push_back(Vec3(2 * std::cos(a), 22 + 2 * std::sin(a), 0));
    }
    path.push_back(Vec3(-2, 10, 0));                          /* inner left leg, down */
}

/* A tube of radius r along a planar path: a ring of n vertices about each
 * point (the frame: the plane's normal and the tangent's in-plane
 * perpendicular), quads between rings, a fan closing each end. Vertex
 * normals point out of the tube, and so does the winding (the renderer
 * takes a hit on a face wound the other way for its back and turns the
 * vertex normal around with it). */
static void Tube(const std::vector<Vec3> &path, float r, int n)
{
    const float pi = 3.14159265f;
    const Vec3 planeN(0, 0, 1);
    const size_t m = path.size();
    /* x, y, z per vertex, as floats: a std::vector of RtPoint (an array
     * type) does not compile with Apple's libc++ */
    std::vector<float> P(3 * (m * n + 2)), N(3 * (m * n + 2));
    for (size_t i = 0; i < m; i++) {
        const Vec3 prev = path[i > 0 ? i - 1 : 0], next = path[i + 1 < m ? i + 1 : m - 1];
        const Vec3 t = normalize(next - prev);
        const Vec3 b = cross(t, planeN); /* in the plane, perpendicular to the wire */
        for (int k = 0; k < n; k++) {
            const float a = 2 * pi * k / n;
            const Vec3 d = planeN * std::cos(a) + b * std::sin(a);
            const Vec3 p = path[i] + d * r;
            p.store(&P[3 * (i * n + k)]);
            d.store(&N[3 * (i * n + k)]);
        }
    }
    const Vec3 t0 = normalize(path[0] - path[1]), t1 = normalize(path[m - 1] - path[m - 2]);
    (path[0] + t0 * r).store(&P[3 * (m * n)]);
    t0.store(&N[3 * (m * n)]);
    (path[m - 1] + t1 * r).store(&P[3 * (m * n + 1)]);
    t1.store(&N[3 * (m * n + 1)]);

    std::vector<RtInt> idx;
    for (size_t i = 0; i + 1 < m; i++)
        for (int k = 0; k < n; k++) {
            const RtInt a = (RtInt)(i * n + k), b = (RtInt)(i * n + (k + 1) % n);
            const RtInt c = (RtInt)((i + 1) * n + k), d = (RtInt)((i + 1) * n + (k + 1) % n);
            idx.push_back(a); idx.push_back(d); idx.push_back(c);
            idx.push_back(a); idx.push_back(b); idx.push_back(d);
        }
    for (int k = 0; k < n; k++) { /* the end caps */
        idx.push_back((RtInt)(m * n)); idx.push_back((RtInt)k); idx.push_back((RtInt)((k + 1) % n));
        idx.push_back((RtInt)(m * n + 1)); idx.push_back((RtInt)((m - 1) * n + (k + 1) % n)); idx.push_back((RtInt)((m - 1) * n + k));
    }
    rtTriangleMesh((RtInt)(P.size() / 3), (const RtPoint *)P.data(), (const RtNormal *)N.data(), NULL, NULL,
                   (RtInt)(idx.size() / 3), idx.data());
}

/* The lamp is hidden from camera rays (it would hang in the frame) and
 * casts no shadow of itself; bounce and shadow rays find it. */
static RtInt LAMP(RtShadeContext *c) { return c->depth > 0 ? 1 : 0; }

void Paperclip(void)
{
    /* The camera: 17 cm from the clip, 5 cm above the table, looking down
     * at it at 16 degrees, low enough that the table's reflection of the
     * clip stretches toward the camera and fades out; a 22 degree lens.
     * Meters. */
    RtPoint eye = {0.01f, 0.05f, 0.17f}, lookat = {0, 0.012f, 0.03f};
    const float fovy = 22;
    RtVector up = {0, 1, 0};
    const float pi = 3.14159265f;

    /* The clip: brushed steel, a wide glossy lobe (specular 0.5,
     * roughness 0.5) over a dark base, metal rather than mirror; wide, so
     * the lamp is narrower than its lobe and is sampled on it, not found
     * by its ray. The table: a flat-sawn
     * oak plank under a clear coat (specular 1, roughness 0.21: the
     * clip's reflection is sharp at its foot and blurs as it stretches
     * toward the camera, until the wood shows through); the pith 11 cm
     * below the top, 15 degrees off its length. The wall: dark grey,
     * matte, 90 cm back. */
    RtMaterial steel = {{0.05f, 0.05f, 0.05f}, {0.5f, 0.5f, 0.48f}, 0.5f, 1, 0, 1.5f, {0, 0, 0}};
    RtMaterial table = {{0.5f, 0.3f, 0.1f}, {1, 1, 1}, 0.21f, 0, 0, 1.5f, {0, 0, 0}};
    WoodParams oak = {{0.44f, 0.25f, 0.095f}, {0.19f, 0.085f, 0.034f}, 18, 1.5f, -2.0f, 15, 1.0f, 0.15f, 0.15f, 0.0005f,
                      {0.01f, 0.05f, 0.17f}, 2 * std::tan(fovy / 2 * pi / 180) / 720, {1, 1, 1}, 0.21f};
    RtMaterial wall = MATTE(0.04f, 0.038f, 0.034f);
    /* The lamp: a 5 cm disc 28 cm up, in front of the clip and to the
     * camera's left, turned toward it, so the shadow falls back and to
     * the right, a few millimeters soft, while the table's reflection
     * of the clip lies in front of it, toward the camera: the two never
     * cross. No fill: the shadow is the point. A grey surround (the
     * background) is the room the steel reflects; the lamp is not where
     * the table would reflect it. */
    RtMaterial key = EMITTER(224, 218, 202);
    const Vec3 lamp(-0.14f, 0.28f, 0.16f), toClip = normalize(Vec3(0, 0.015f, 0) - lamp);
    const Vec3 lampA = normalize(cross(Vec3(0, 1, 0), toClip)), lampB = cross(toClip, lampA); /* a x b = toClip */
    RtColor ka = {0.008f, 0.009f, 0.008f};
    RtColor room = {0.5f, 0.5f, 0.53f};

    rtBegin();
        rtFormat(960, 720);
        rtDisplay("1_TestScenes/Paperclip");
        rtTonemap("filmic", 0.0f);
        rtCamera(eye, lookat, up, fovy);
        rtPixelSamples(512);
        rtSampler("halton");
        rtMaxDepth(4);
        rtAmbient(ka);
        rtBackground(room);
        rtWorldBegin(0);
            rtCutout(LAMP, NULL, 0);
            rtMaterial(&key);
            Disc(lamp, lampA, lampB, 0.025f);
            rtCutout(NULL, NULL, 0);

            rtMaterial(&table);
            rtSurface(WOOD, &oak, sizeof oak);
            Quad(Vec3(-0.4f, 0, -0.12f), Vec3(0.8f, 0, 0), Vec3(0, 0, 0.52f)); /* faces up; its far edge 12 cm behind the clip */
            rtSurface(NULL, NULL, 0);
            rtMaterial(&wall);
            Quad(Vec3(-1.5f, -0.5f, -0.9f), Vec3(0, 2, 0), Vec3(3, 0, 0)); /* faces the camera */

            rtMaterial(&steel);
            rtTransformBegin();
                rtRotate(-8, 0, 1, 0); /* turned a little toward the camera */
                rtScale(0.001f, 0.001f, 0.001f); /* the clip is in millimeters */
                std::vector<Vec3> path;
                ClipPath(path);
                Tube(path, 0.5f, 10);
            rtTransformEnd();
        rtWorldEnd();
    rtEnd();
}
