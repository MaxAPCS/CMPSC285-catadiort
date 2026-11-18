/*
 * ECE/CS 285 – HW3 Test Scene: Stripes and Stripes_halton
 * Black and white stripes from 4 pixels wide down to half a pixel, on a
 * quad that fills the frame, lit by the ambient term alone: vertical on
 * the left third, tilted 30 degrees in the middle, horizontal on the
 * right. Tests the stratified (Stripes) and halton (Stripes_halton)
 * samplers on the pixel's area alone.
 * 960 x 540, 64 spp, stratified or halton, seed 0.
 *
 * Output: renders/<integrator>/1_TestScenes/Stripes.png, Stripes_halton.png
 */
#include "rt.h"
#include "shaders.h"
#include "sceneutil.h"

#include <cmath>

/* The camera looks down -z from the origin; the quad is 1 m out. */
static const float FOVY = 30.0f;
static const int WIDTH = 960, HEIGHT = 540;

/* The stripes' phase t pixels into a sweep of length len: the frequency
 * rises from 1/8 cycle a pixel (stripes 4 pixels wide) to 1 (half a
 * pixel) by the same factor every pixel, so each octave gets equal room. */
static float sweep(float t, float len)
{
    const float f0 = 0.125f, k = std::log(8.0f) / len;
    return f0 / k * (std::exp(k * t) - 1);
}

/* The quad's albedo at the hit, from the hit's pixel coordinates in the
 * frame (x right, y down, from the top left corner). */
static void STRIPES(RtShadeContext *c)
{
    const float h = std::tan(FOVY * 0.5f * 3.14159265f / 180), w = h * WIDTH / HEIGHT;
    const float px = (c->P[0] + w) / (2 * h) * HEIGHT, py = (h - c->P[1]) / (2 * h) * HEIGHT;
    const float third = WIDTH / 3.0f, cos30 = 0.8660254f;
    float phase;
    if (px < third) {
        phase = sweep(px, third); /* vertical stripes */
    } else if (px < 2 * third) {
        phase = sweep((px - third) * cos30 + py * 0.5f, third * cos30 + HEIGHT * 0.5f); /* tilted 30 degrees */
    } else {
        phase = sweep(py, (float)HEIGHT); /* horizontal stripes */
    }
    const float g = (phase - std::floor(phase) < 0.5f) ? 0.9f : 0.04f;
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = g;
    }
}

static void stripes(const char *sampler, const char *display)
{
    RtPoint eye = {0, 0, 0}, lookat = {0, 0, -1};
    RtVector up = {0, 1, 0};
    RtColor ka = {1, 1, 1};
    RtMaterial white = MATTE(0.9f, 0.9f, 0.9f);
    const float h = std::tan(FOVY * 0.5f * 3.14159265f / 180), w = h * WIDTH / HEIGHT;

    rtBegin();
        rtFormat(WIDTH, HEIGHT);
        rtDisplay(display);
        rtCamera(eye, lookat, up, FOVY);
        rtPixelSamples(64);
        rtSampler(sampler);
        rtAmbient(ka);
        rtWorldBegin(0);
            /* a tenth wider and taller than the frame, so it fills it */
            rtMaterial(&white);
            rtSurface(STRIPES, NULL, 0);
            Quad(Vec3(-1.1f * w, 1.1f * h, -1), Vec3(2.2f * w, 0, 0), Vec3(0, -2.2f * h, 0));
        rtWorldEnd();
    rtEnd();
}

void Stripes(void) { stripes("stratified", "1_TestScenes/Stripes"); }
void Stripes_halton(void) { stripes("halton", "1_TestScenes/Stripes_halton"); }
