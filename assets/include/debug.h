/*
 * debug.h: the debugging aids (Workflow, steps 5 and 16). Complete;
 * nothing to change. --trace X Y logs what the renderer does for one
 * pixel, and the encoders below turn what a renderer knows at a hit into
 * the --aov images, so every renderer writes the same pictures.
 */
#ifndef DEBUG_H
#define DEBUG_H

/* --trace X Y: the pixel loop calls traceBegin(px, py) before a pixel's
 * samples and traceEnd() after them; in between, RT_TRACE(...) prints one
 * line on stderr (printf's format) when that pixel is the traced one.
 * Print at the points the Whitted Illumination handout names, in its
 * format, so your log diffs against the reference's (gauchoRT Spheres
 * --trace 320 240 with the reference library). Only the worker rendering
 * the traced pixel prints, so lines never interleave, and on every other
 * pixel a RT_TRACE is one test of a thread-local flag, before its
 * arguments are even evaluated: free in the render loop (a macro, so the
 * test is inline). */
extern thread_local bool traceOn; /* this thread is on the traced pixel */
void traceBegin(int px, int py);
void traceEnd();
inline bool tracing() { return traceOn; }
#if defined(__GNUC__) || defined(__clang__)
__attribute__((format(printf, 1, 2)))
#endif
void traceLine(const char *format, ...);
#define RT_TRACE(...)                       \
    do {                                 \
        if (traceOn)                     \
            traceLine(__VA_ARGS__);      \
    } while (0)

/* The --aov images, each a color from what the renderer has at the first
 * hit: the camera ray's direction (raydir, before any
 * hit) or the shading or geometric normal as 0.5 n + 0.5,
 * depth as 1 / t, (u, v, 0), the albedo as it is, the camera ray's cost
 * (BVH nodes and primitives tested) over 512, and the fraction of the
 * scene's lights whose shadow ray is unblocked (shadow). Each writes
 * rgb[3]. */
inline void aovNormalColor(const float n[3], float rgb[3])
{
    for (int i = 0; i < 3; i++)
        rgb[i] = 0.5f * n[i] + 0.5f;
}
inline void aovDepthColor(float t, float rgb[3]) { rgb[0] = rgb[1] = rgb[2] = t > 0 ? 1 / t : 0; }
inline void aovUvColor(float u, float v, float rgb[3])
{
    rgb[0] = u;
    rgb[1] = v;
    rgb[2] = 0;
}
inline void aovCostColor(int cost, float rgb[3]) { rgb[0] = rgb[1] = rgb[2] = (float)cost / 512.0f; }
inline void aovShadowColor(int unblocked, int lights, float rgb[3])
{
    rgb[0] = rgb[1] = rgb[2] = lights > 0 ? (float)unblocked / (float)lights : 0;
}

#endif /* DEBUG_H */
