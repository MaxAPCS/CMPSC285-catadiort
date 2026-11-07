/*
 * ECE/CS 285 – HW2 Benchmark Scene: Dragon
 * The Stanford dragon (871,414 triangles, vertex normals) as slightly
 * cloudy glass (albedo 0.05, kt 0.85, ior 1.5, max depth 10) on a
 * CHECKER floor in front of a framed mirror, under two point lights and
 * a pale warm background: deep refraction through a dense mesh, two rays
 * spawned at every glass surface; 256 spp, halton, seed 0 under cook and
 * kajiya.
 * Model: the Stanford Computer Graphics Laboratory's dragon (Curless and
 * Levoy, 1996), models/dragon/dragon.mesh, from dragon_vrip.ply.
 *
 * Output: renders/<integrator>/2_BenchmarkScenes/Dragon.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

void Dragon(void)
{
    RtPoint eye = {-2.6f, 1.5f, 3.6f}, lookat = {0.3f, 0.7f, -0.2f}; /* off to the side, so the mirror shows the dragon's other flank */
    RtVector up = {0, 1, 0};
    RtPoint keyPos = {-3, 5, 4}, fillPos = {4, 3, 2};
    RtColor keyI = {90, 90, 90}, fillI = {40, 40, 40}; /* W/sr */
    RtColor ka = {0.12f, 0.12f, 0.12f};
    CheckerParams checks = {8, 8, MATTE(0.85f, 0.85f, 0.8f), MATTE(0.3f, 0.3f, 0.35f)};
    RtMaterial glass = GLASS, mirror = MIRROR, frame = MATTE(0.08f, 0.05f, 0.03f);
    /* Cloudy glass: a little diffuse albedo shows the body's shape where a
     * perfectly clear one would be invisible; kt 0.85 leaves it room. */
    glass.albedo[0] = 0.05f;
    glass.albedo[1] = 0.06f;
    glass.albedo[2] = 0.07f;
    glass.transmission = 0.85f;

    rtBegin();
        rtFormat(1920, 1080);
        rtDisplay("2_BenchmarkScenes/Dragon");
        rtPixelSamples(256); /* cook and kajiya; whitted renders one sample */
        rtSampler("halton");
        rtCamera(eye, lookat, up, 32);
        rtAmbient(ka);
        rtMaxDepth(10);
        rtBackground(Color8(226, 218, 205)); /* a pale warm studio backdrop; the glass reflects it */
        rtWorldBegin(0);
            rtPointLight(keyPos, keyI);
            rtPointLight(fillPos, fillI);

            rtSurface(CHECKER, &checks, sizeof checks);
            Floor(8); /* 8 m, 1 m checks */
            rtSurface(NULL, NULL, 0);

            /* The mirror: 5 m wide, 2.6 m tall in a dark frame, 1.4 m
             * behind the dragon and nearly facing the camera: the dragon's
             * other flank appears in it, large, 20 degrees to the left of
             * the dragon itself. */
            rtTransformBegin();
                rtTranslate(-0.6f, 0, -1.4f);
                rtRotate(5, 0, 1, 0);
                rtMaterial(&frame);
                Box(Vec3(-2.6f, 0, -0.08f), Vec3(2.6f, 2.75f, 0));
                rtMaterial(&mirror);
                Quad(Vec3(-2.5f, 2.65f, 0.001f), Vec3(5, 0, 0), Vec3(0, -2.55f, 0));
            rtTransformEnd();

            /* The scan is 0.144 m tall with its feet at y = 0.053; scaled by
             * 10 it stands 1.44 m, centered on x. */
            ObjMesh *dragon = obj_load("models/dragon/dragon.mesh");
            rtMaterial(&glass);
            rtTransformBegin();
                rtRotate(-20, 0, 1, 0);
                rtScale(10, 10, 10);
                rtTranslate(0.006f, -0.053f, 0.005f);
                ObjDraw(dragon, OBJ_KEEP_MATERIAL);
            rtTransformEnd();
            obj_free(dragon);
        rtWorldEnd();
    rtEnd();
}
