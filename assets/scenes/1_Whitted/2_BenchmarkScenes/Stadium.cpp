/*
 * ECE/CS 285 – HW2 Benchmark Scene: Stadium
 * The Stanford dragon, matte, 3 m tall in an arena built in a loop here:
 * a 400 m floor, a walkway and a hundred tiers of bleachers, each a ring
 * of sixteen quads up to 60 m long, under one sun and a sky. 871,414 tiny
 * triangles and 3,200 huge ones in one tree, the quads' boxes enormous
 * for their area: the gap between the SAH and a midpoint split; 256 spp,
 * halton, seed 0 under cook and kajiya.
 * Model: the Stanford dragon (see Dragon).
 *
 * Output: renders/<integrator>/2_BenchmarkScenes/Stadium.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

#include <cmath>

/* One quad from four corners, counterclockwise seen from the front. */
static void Patch(const Vec3 &a, const Vec3 &b, const Vec3 &c, const Vec3 &d)
{
    RtPoint P[4];
    a.store(P[0]);
    b.store(P[1]);
    c.store(P[2]);
    d.store(P[3]);
    const RtInt indices[6] = {0, 1, 2, 0, 2, 3};
    rtTriangleMesh(4, P, NULL, NULL, NULL, 2, indices);
}

void Stadium(void)
{
    RtPoint eye = {5.5f, 3.6f, 8.5f}, lookat = {0, 2.0f, 0};
    RtVector up = {0, 1, 0};
    RtVector sunDir = {-0.55f, -0.7f, -0.6f}; /* a late sun from behind the camera's right shoulder */
    RtColor sunE = {3.6f, 3.1f, 2.4f};
    RtColor ka = {0.22f, 0.2f, 0.2f};
    RtMaterial grass = MATTE(0.25f, 0.5f, 0.2f);
    RtMaterial scales = MATTE(0.75f, 0.7f, 0.6f);
    RtMaterial red = MATTE(0.7f, 0.12f, 0.1f), blue = MATTE(0.1f, 0.2f, 0.65f), concrete = MATTE(0.55f, 0.55f, 0.5f);
    RtMaterial redSeat = MATTE(0.55f, 0.09f, 0.08f), blueSeat = MATTE(0.08f, 0.15f, 0.5f); /* darker seat tops, so the tiers read as rows */
    /* concrete is kept for a grey walkway between the field and the stands */
    const float pi = 3.14159265358979323846f;

    rtBegin();
        rtFormat(1920, 1080);
        rtDisplay("2_BenchmarkScenes/Stadium");
        rtPixelSamples(256); /* cook and kajiya; whitted renders one sample */
        rtSampler("halton");
        rtCamera(eye, lookat, up, 40);
        rtAmbient(ka);
        rtBackground(Color8(236, 204, 168)); /* a late-afternoon sky */
        rtWorldBegin(0);
            rtDirectionalLight(sunDir, sunE);

            rtMaterial(&grass);
            Floor(400);

            /* A grey walkway ring between the field and the first tier. */
            rtMaterial(&concrete);
            for (int s = 0; s < 16; s++) {
                const float a0 = 2 * pi * s / 16, a1 = 2 * pi * (s + 1) / 16;
                Patch(Vec3(36 * std::cos(a0), 0.01f, 36 * std::sin(a0)), Vec3(36 * std::cos(a1), 0.01f, 36 * std::sin(a1)),
                      Vec3(40 * std::cos(a1), 0.01f, 40 * std::sin(a1)), Vec3(40 * std::cos(a0), 0.01f, 40 * std::sin(a0)));
            }

            /* The bleachers: tier t is a ring at radius 40 + 1.2 t, rising
             * 1 m per tier, made of sixteen seat quads (flat) and sixteen
             * risers (vertical), each segment in its own color so the
             * stands read as blocks of red and blue from the field. */
            const int tiers = 100, segments = 16;
            for (int t = 0; t < tiers; t++) {
                const float r0 = 40 + 1.2f * t, r1 = r0 + 1.2f, y0 = 1.0f * t, y1 = y0 + 1.0f;
                for (int s = 0; s < segments; s++) {
                    const float a0 = 2 * pi * s / segments, a1 = 2 * pi * (s + 1) / segments;
                    const float c0 = std::cos(a0), s0 = std::sin(a0), c1 = std::cos(a1), s1 = std::sin(a1);
                    rtMaterial(s % 2 ? &redSeat : &blueSeat);
                    Patch(Vec3(r0 * c0, y1, r0 * s0), Vec3(r0 * c1, y1, r0 * s1), Vec3(r1 * c1, y1, r1 * s1),
                          Vec3(r1 * c0, y1, r1 * s0)); /* the seat, facing up */
                    rtMaterial(s % 2 ? (t % 2 ? &red : &redSeat) : (t % 2 ? &blue : &blueSeat)); /* rows alternate in shade */
                    Patch(Vec3(r0 * c1, y0, r0 * s1), Vec3(r0 * c0, y0, r0 * s0), Vec3(r0 * c0, y1, r0 * s0),
                          Vec3(r0 * c1, y1, r0 * s1)); /* the riser, facing the field */
                }
            }

            /* The dragon: the scan is 0.144 m tall; scaled by 21 it stands
             * 3 m, facing the camera's side. */
            ObjMesh *dragon = obj_load("models/dragon/dragon.mesh");
            rtMaterial(&scales);
            rtTransformBegin();
                rtRotate(140, 0, 1, 0); /* facing the camera's side */
                rtScale(21, 21, 21);
                rtTranslate(0.006f, -0.053f, 0.005f);
                ObjDraw(dragon, OBJ_KEEP_MATERIAL);
            rtTransformEnd();
            obj_free(dragon);
        rtWorldEnd();
    rtEnd();
}
