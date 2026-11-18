/*
 * ECE/CS 285 – HW3 Test Scene: GlossyFurnace
 * A sphere with no albedo, specular 1 and roughness 0.548 (alpha 0.3,
 * Phong exponent 20.2) in a closed box whose walls emit radiance 1: every
 * reflected ray finds radiance 1, so each pixel of the sphere is the
 * lobe's directional albedo, which checks the lobe's weight (ks, no
 * cosine) and its sampler together; it falls toward the rim, where part
 * of the lobe dips below the surface. 480 x 480, 256 spp, independent,
 * seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/GlossyFurnace.png
 */

#include "rt.h"
#include "sceneutil.h"

void GlossyFurnace(void)
{
    RtPoint eye = {0, 0, 4}, lookat = {0, 0, 0};
    RtVector up = {0, 1, 0};
    RtColor black = {0, 0, 0};
    RtMaterial walls = EMITTER(1, 1, 1);
    RtMaterial glossy = {{0, 0, 0}, {1, 1, 1}, 0.548f, 0, 0, 1.5f, {0, 0, 0}};

    rtBegin();
        rtFormat(480, 480);
        rtDisplay("1_TestScenes/GlossyFurnace");
        rtCamera(eye, lookat, up, 40);
        rtPixelSamples(256);
        rtSampler("independent");
        rtAmbient(black);
        rtBackground(black);
        rtWorldBegin(0);
            rtMaterial(&walls);
            Box(Vec3(-6, -6, -6), Vec3(6, 6, 6), true); /* six emitters facing in */
            rtMaterial(&glossy);
            rtSphere(1);
        rtWorldEnd();
    rtEnd();
}
