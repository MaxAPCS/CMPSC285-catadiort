/*
 * ECE/CS 285 – HW2 Test Scene: Aliasing
 * A checker floor, 0.5 m checks, running a kilometer to the horizon, and
 * a Siemens star of 96 black and 96 white spokes on a white quad, under
 * one directional light and a little ambient. The far checks and the
 * star's center are finer than a pixel: at 1 spp (Aliasing) they break
 * into noise where whitted shows moiré; at 64 spp (Aliasing_64) they
 * average to a smooth grey. 960 x 540, stratified, seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/Aliasing.png, Aliasing_64.png
 */

#include "rt.h"
#include "shaders.h"
#include "sceneutil.h"

#include <cmath>

/* The Siemens star: SPOKE_PAIRS black spokes and as many white ones,
 * alternating around the center of the quad's (u, v) square, inside a
 * disk of radius 0.47; the rest of the quad is white. The quad is bound
 * white, so the shader only darkens the black spokes. */
static const int SPOKE_PAIRS = 96;

static void SPOKES(RtShadeContext *c)
{
    const float x = c->u - 0.5f, y = 0.5f - c->v;
    const float r = std::sqrt(x * x + y * y);
    if (r > 0.47f)
        return;
    const float a = std::atan2(y, x) / 3.14159265358979f; /* (-1, 1] turns of a half circle */
    const int sector = (int)std::floor((a + 1) * SPOKE_PAIRS);
    if (sector & 1)
        for (int i = 0; i < 3; i++)
            c->m.albedo[i] = 0.04f;
}

static void aliasing(int spp, const char *display)
{
    RtPoint eye = {0, 1.6f, 6}, lookat = {0, 0.9f, 0};
    RtVector up = {0, 1, 0};
    RtVector sunDir = {-0.3f, -0.8f, -0.6f};
    RtColor sunE = {2.5f, 2.5f, 2.5f}; /* W/m^2 */
    RtColor ka = {0.3f, 0.3f, 0.3f};

    RtMaterial white = MATTE(0.9f, 0.9f, 0.9f);
    CheckerParams checks = {4000, 4000, MATTE(0.9f, 0.9f, 0.9f), MATTE(0.04f, 0.04f, 0.04f)};

    rtBegin();
        rtFormat(960, 540);
        rtDisplay(display);
        rtCamera(eye, lookat, up, 35);
        rtPixelSamples(spp);
        rtSampler("stratified");
        rtAmbient(ka);
        rtBackground(Color(0.55f, 0.65f, 0.85f));
        rtWorldBegin(0);
            rtDirectionalLight(sunDir, sunE);

            /* The floor: 2 km across, so its far edge sits 0.1 degrees
             * below the true horizon, with 4000 checks a side. */
            rtMaterial(&white);
            rtSurface(CHECKER, &checks, sizeof checks);
            Floor(2000);

            /* The star on a 2.4 m quad standing on the floor, left of
             * center so the floor runs to the horizon on the right. */
            rtSurface(SPOKES, NULL, 0);
            Quad(Vec3(-2.2f, 2.4f, -1), Vec3(2.4f, 0, 0), Vec3(0, -2.4f, 0));
        rtWorldEnd();
    rtEnd();
}

void Aliasing(void) { aliasing(1, "1_TestScenes/Aliasing"); }
void Aliasing_64(void) { aliasing(64, "1_TestScenes/Aliasing_64"); }
