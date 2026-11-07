/*
 * sceneutil.h: what the scene files share. Colors from floats or 8-bit
 * sRGB, the named materials the test scenes use (the same names reappear
 * in HW3 and HW4), a quad, a floor, a box and loose triangles as meshes,
 * and an octave sum of perlinNoise. Like obj.h it is C++ for the scene
 * files only; the renderer never calls it, and the interface (rt.h) never
 * requires vec.h.
 */
#ifndef SCENEUTIL_H
#define SCENEUTIL_H

#include "rt.h"
#include "shaders.h"
#include "vec.h"

#include <algorithm>
#include <cmath>
#include <vector>

/* colors --------------------------------------------------------------- */

/* An RtColor built from a call and passed to anything taking an RtColor:
 *     rtBackground(Color8(67, 178, 221));
 *     rtAmbient(Color(0.1f, 0.1f, 0.1f));
 * Color8 takes 8-bit sRGB components, 0 to 255, as a color picker shows
 * them, decoded to linear; Color takes linear values as they are. */
struct Rgb {
    RtColor v;
    operator RtFloat *() { return v; }
    operator const RtFloat *() const { return v; }
};

inline Rgb Color(float r, float g, float b)
{
    Rgb c = {{r, g, b}};
    return c;
}

/* One 8-bit sRGB component as linear. */
inline float Srgb8ToLinear(int byte)
{
    const float s = (float)byte / 255.0f;
    return s <= 0.04045f ? s / 12.92f : std::pow((s + 0.055f) / 1.055f, 2.4f);
}

inline Rgb Color8(int r, int g, int b) { return Color(Srgb8ToLinear(r), Srgb8ToLinear(g), Srgb8ToLinear(b)); }

/* materials ------------------------------------------------------------ */

/* The named materials (rt.h, RtMaterial); a scene copies one and passes
 * its address (RtMaterial red = MATTE(...); rtMaterial(&red)). Under
 * whitted (Whitted Illumination handout) a nonzero specular gives a
 * highlight, and a mirror ray only when smooth (roughness below about
 * 0.4, alpha = roughness^2 below 0.15): MIRROR and GLASS reflect, PLASTIC
 * (0.55) has a highlight only, MATTE has neither. */

/* A diffuse surface of the given albedo: no highlight, no mirror ray. */
inline RtMaterial MATTE(float r, float g, float b)
{
    RtMaterial m = {{r, g, b}, {0, 0, 0}, 1.0f, 0, 0, 1.5f, {0, 0, 0}};
    return m;
}

inline RtMaterial MATTE(const Vec3 &albedo) { return MATTE(albedo.x, albedo.y, albedo.z); }
inline RtMaterial MATTE(const Rgb &albedo) { return MATTE(albedo.v[0], albedo.v[1], albedo.v[2]); }

/* A glossy surface: the albedo plus a weak white specular, medium
 * roughness: a broad highlight (and, from HW3, a blurred reflection). */
inline RtMaterial PLASTIC(float r, float g, float b)
{
    RtMaterial m = {{r, g, b}, {0.08f, 0.08f, 0.08f}, 0.55f, 0, 0, 1.5f, {0, 0, 0}};
    return m;
}

inline RtMaterial PLASTIC(const Vec3 &albedo) { return PLASTIC(albedo.x, albedo.y, albedo.z); }
inline RtMaterial PLASTIC(const Rgb &albedo) { return PLASTIC(albedo.v[0], albedo.v[1], albedo.v[2]); }

/* A mirror: no albedo, specular 0.9, perfectly smooth, and metallic, which
 * whitted and cook ignore and kajiya (HW4) reads: specular is the
 * reflectance head on under all three (the mirror ray's weight, the
 * glossy lobe's, Schlick's F0), and metallic says there is no diffuse
 * under it. */
static const RtMaterial MIRROR = {{0, 0, 0}, {0.9f, 0.9f, 0.9f}, 0, 1, 0, 1.5f, {0, 0, 0}};

/* Clear glass: no albedo, specular 1, transmission 1, ior 1.5 (the Fresnel
 * split between the two is the integrator's). */
static const RtMaterial GLASS = {{0, 0, 0}, {1, 1, 1}, 0, 0, 1, 1.5f, {0, 0, 0}};

/* The grey checker: n x n checks of light and dark grey (sRGB 235 and
 * 120), matte; the course's test floor. */
inline CheckerParams GRAY_CHECKER(int n)
{
    CheckerParams p = {n, n, MATTE(Color8(235, 235, 235)), MATTE(Color8(120, 120, 120))};
    return p;
}

/* A light: emission of the given radiance (W/m^2/sr) from the front of
 * every primitive drawn with it, and no albedo. Each draw call made with
 * it is one area light (two lamps need two calls): a stand-in point light
 * under whitted, its surface sampled under cook and kajiya (the Whitted
 * Illumination handout, and the HW3 and HW4 handouts). A quad of area A
 * and radiance Le sends Le A W/sr straight out, as a point light of
 * intensity Le A would. */
inline RtMaterial EMITTER(float r, float g, float b)
{
    RtMaterial m = {{0, 0, 0}, {0, 0, 0}, 1.0f, 0, 0, 1.5f, {r, g, b}};
    return m;
}

/* geometry ------------------------------------------------------------- */

/* A quad as two triangles with normals and texture coordinates. o is the
 * corner with (s, t) = (0, 0), s runs along eu and t along ev (eu is the
 * texture's right, ev its down); the normal is ev x eu. A floor seen from
 * above is Quad(o, +x, +z); a wall facing a camera at +z is
 * Quad(upper-left, +x, -y). */
inline void Quad(const Vec3 &o, const Vec3 &eu, const Vec3 &ev)
{
    Vec3 n = normalize(cross(ev, eu));
    Vec3 corners[4] = {o, o + ev, o + ev + eu, o + eu};
    RtPoint P[4];
    RtNormal N[4];
    for (int i = 0; i < 4; i++) {
        corners[i].store(P[i]);
        n.store(N[i]);
    }
    const RtFloat st[8] = {0, 0, 0, 1, 1, 1, 1, 0};
    const RtInt indices[6] = {0, 1, 2, 0, 2, 3}; /* counterclockwise from the front */
    rtTriangleMesh(4, P, N, st, NULL, 2, indices);
}

/* A square floor `size` across, centered on the origin at y = 0, facing up;
 * (s, t) runs from (0, 0) at the far left corner (-x, -z) to (1, 1) at the
 * near right one, so a texture on it reads upright from a camera at +z. */
inline void Floor(float size)
{
    Quad(Vec3(-size / 2, 0, -size / 2), Vec3(size, 0, 0), Vec3(0, 0, size));
}

/* An axis-aligned box from corner lo to hi: six quads, normals outward, or
 * inward if `inward` (a room seen from inside; a Cornell box is
 * Box(lo, hi, true) minus the front wall). Each face's texture reads
 * upright from outside, top row at the top of the box. */
inline void Box(const Vec3 &lo, const Vec3 &hi, bool inward = false)
{
    const float w = hi.x - lo.x, h = hi.y - lo.y, d = hi.z - lo.z;
    const Vec3 X(w, 0, 0), Y(0, h, 0), Z(0, 0, d);
    struct Face {
        Vec3 o, eu, ev;
    } faces[6] = {
        {Vec3(lo.x, hi.y, hi.z), X, -Y},  /* +z, the front */
        {Vec3(hi.x, hi.y, lo.z), -X, -Y}, /* -z, the back */
        {Vec3(hi.x, hi.y, hi.z), -Z, -Y}, /* +x, the right side */
        {Vec3(lo.x, hi.y, lo.z), Z, -Y},  /* -x, the left side */
        {Vec3(lo.x, hi.y, lo.z), X, Z},   /* +y, the top */
        {Vec3(lo.x, lo.y, hi.z), X, -Z},  /* -y, the bottom */
    };
    for (const Face &f : faces) {
        if (inward)
            Quad(f.o, f.ev, f.eu); /* swapping the edges flips the normal */
        else
            Quad(f.o, f.eu, f.ev);
    }
}

/* ntris triangles given as three corners each, nothing shared: for small
 * hand-made meshes (a box's faces listed corner by corner). No normals, so
 * each triangle is flat. */
inline void Triangles(const RtPoint *corners, int ntris)
{
    std::vector<RtInt> indices((size_t)ntris * 3);
    for (int i = 0; i < 3 * ntris; i++)
        indices[(size_t)i] = i;
    rtTriangleMesh(3 * ntris, corners, NULL, NULL, NULL, ntris, indices.data());
}

/* A disc of radius r about c in the plane of a and b (unit, orthogonal),
 * as a fan of n flat triangles; its front faces a x b. */
inline void Disc(const Vec3 &c, const Vec3 &a, const Vec3 &b, float r, int n = 32)
{
    /* x, y, z of three corners per triangle, as floats: a std::vector of
     * RtPoint (an array type) does not compile with Apple's libc++ */
    std::vector<float> corners((size_t)n * 9);
    for (int i = 0; i < n; i++) {
        const float t0 = 2 * 3.14159265f * (float)i / (float)n, t1 = 2 * 3.14159265f * (float)(i + 1) / (float)n;
        c.store(&corners[(size_t)i * 9]);
        (c + a * (r * std::cos(t0)) + b * (r * std::sin(t0))).store(&corners[(size_t)i * 9 + 3]);
        (c + a * (r * std::cos(t1)) + b * (r * std::sin(t1))).store(&corners[(size_t)i * 9 + 6]);
    }
    Triangles((const RtPoint *)corners.data(), n);
}

/* A sun of irradiance E (on a facing surface) and angular diameter
 * `degrees`, traveling along dir, as an emitting disc `distance` from
 * `about` (far enough that the whole scene sees it the same way) that
 * subtends that angle: radiance E / (pi sin^2(degrees / 2)), the disc's
 * irradiance on its axis, so cook and kajiya get E with soft shadows;
 * whitted's stand-in point light (intensity Le x area) delivers E over
 * cos^2(degrees / 2), 3% more for a 20 degree sun. Binds the emitter
 * material. */
inline void SunDisc(const Vec3 &dir, const RtColor E, float degrees, const Vec3 &about, float distance = 50.0f)
{
    const float half = 0.5f * degrees * 3.14159265f / 180;
    const float D = distance, r = D * std::tan(half);
    const float omega = 3.14159265f * std::sin(half) * std::sin(half);
    RtMaterial m = EMITTER(E[0] / omega, E[1] / omega, E[2] / omega);
    rtMaterial(&m);
    const Vec3 a = normalize(cross(dir, std::fabs(dir.y) < 0.9f ? Vec3(0, 1, 0) : Vec3(1, 0, 0)));
    const Vec3 b = cross(dir, a); /* a x b = dir: the disc faces the scene */
    Disc(about - dir * D, a, b, r);
}

/* cameras -------------------------------------------------------------- */

/* A camera in photographic terms, turned into the two numbers rt.h takes:
 * a lens of focal length focalMM on a sensor sensorHeightMM tall (24 for
 * a landscape full frame, 36 for a portrait one) gives the vertical field
 * of view, 2 atan(sensor / 2 f), and an f-number the lens radius,
 * f / (2 N), in meters. */
inline float FovyFromFocalLength(float focalMM, float sensorHeightMM = 24.0f)
{
    return 2.0f * std::atan(sensorHeightMM / (2.0f * focalMM)) * (180.0f / 3.14159265358979f);
}
inline float LensRadiusFromFStop(float focalMM, float fNumber) { return focalMM / (2.0f * fNumber) / 1000.0f; }

/* rtCamera and rtLens together, from the photographic numbers; focused at
 * focusDistance meters. An f-number of 0 means a pinhole. */
inline void PhotoCamera(RtPoint eye, RtPoint lookat, RtVector up, float focalMM, float fNumber, float focusDistance,
                        float sensorHeightMM = 24.0f)
{
    rtCamera(eye, lookat, up, FovyFromFocalLength(focalMM, sensorHeightMM));
    rtLens(fNumber > 0 ? LensRadiusFromFStop(focalMM, fNumber) : 0.0f, focusDistance);
}

/* lamps ---------------------------------------------------------------- */

/* The cutout of a lamp's sphere, as a converted scene's point lamp with a
 * radius needs it: the camera does not see it and it casts no shadow
 * (depth <= 0), and a bounce ray from inside leaves it (it is no surface
 * to what it encloses); a bounce ray from outside still finds it, so it
 * is counted with MIS. Draw the sphere under a translation, so Po is the
 * point about its center and a ray going out has I . Po > 0. */
inline RtInt LAMP_SPHERE(RtShadeContext *c)
{
    const RtFloat out = c->I[0] * c->Po[0] + c->I[1] * c->Po[1] + c->I[2] * c->Po[2];
    return c->depth > 0 && out < 0 ? 1 : 0;
}

/* movies --------------------------------------------------------------- */

/* The movies' shutter: a frame's exposure covers this fraction of the
 * step between it and the next frame (SHUTTER, a 180 degree shutter;
 * MoleculeFastMovie exposes the whole step, 1, so its trails join),
 * keyed with rtMotionBegin: whitted renders the frame at its start, and
 * a cook or kajiya with motion blur (HW3 extra credit) blurs it over the
 * exposure; one without it shows the start too. */
static const float SHUTTER = 0.5f;

/* noise ---------------------------------------------------------------- */

/* The renderer's perlinNoise (shaders.h) summed over octaves ("fractional
 * Brownian motion"): each octave about twice as fine (2.03, so the
 * lattices never line up) and half as strong. About [-1, 1]. */
inline double Fbm(double x, double y, double z, int octaves)
{
    double sum = 0, amp = 0.5;
    for (int i = 0; i < octaves; i++, amp *= 0.5, x *= 2.03, y *= 2.03, z *= 2.03)
        sum += amp * perlinNoise(x, y, z);
    return sum;
}

/* wood ----------------------------------------------------------------- */

/* a + (b - a) t */
inline double Mix(double a, double b, double t) { return a + (b - a) * t; }

/* 0 below e0, 1 above e1, a smooth S-curve between (smoothstep) */
inline double Smooth(double e0, double e1, double x)
{
    const double t = std::max(0.0, std::min(1.0, (x - e0) / (e1 - e0)));
    return t * t * (3 - 2 * t);
}

/* smoothstep(e0, e1, x) - smoothstep(e2, e3, x), repeated with period p */
inline double SmoothPulseTrain(double e0, double e1, double e2, double e3, double p, double x)
{
    const double m = x - p * std::floor(x / p);
    return Smooth(e0, e1, m) - Smooth(e2, e3, m);
}

/* Larry Gritz's oak from Advanced RenderMan (Apodaca and Gritz, chapter
 * 12, "oaktexture"), as HW1 had it: growth rings are cylinders about the
 * z axis of P (the log's axis), warped by noise, the trunk wobbling as it
 * goes up and the rings of uneven width; fine pores (the grain) are noise
 * stretched along the log. 0 for light wood, 1 for dark. fw is a pixel's
 * size in P's units: detail finer than that fades out instead of
 * aliasing. grainy scales the pores (1 is Gritz's). */
inline double OakTexture(double Px, double Py, double Pz, double fw, double grainy = 1)
{
    const double ringfreq = 8, ringunevenness = 0.5, grainfreq = 25;
    const double ringnoise = 0.02, ringnoisefreq = 1;
    const double trunkwobble = 0.15, trunkwobblefreq = 0.025;
    const double angularwobble = 0.1, angularwobblefreq = 1.5;

    /* a general warping of the domain: vector fBm, 2 octaves, lacunarity 4 */
    double off[3] = {0, 0, 0}, amp = 1, f = ringnoisefreq;
    for (int o = 0; o < 2; o++, amp *= 0.5, f *= 4) {
        off[0] += amp * perlinNoise(Px * f, Py * f, Pz * f);
        off[1] += amp * perlinNoise(Px * f + 31.7, Py * f + 11.3, Pz * f + 7.1);
        off[2] += amp * perlinNoise(Px * f + 5.9, Py * f + 23.1, Pz * f + 41.3);
    }
    double rx = Px + ringnoise * off[0], ry = Py + ringnoise * off[1];
    const double rz = Pz + ringnoise * off[2];

    /* the trunk is not steady in x and y as it goes up in z */
    rx += trunkwobble * perlinNoise(Pz * trunkwobblefreq, 0.37, 0.71);
    ry += trunkwobble * perlinNoise(Pz * trunkwobblefreq, 4.13, 9.29);

    /* the distance from the center, in rings, with noise around the trunk
     * and rings of unequal width */
    double r = std::sqrt(rx * rx + ry * ry) * ringfreq;
    r += angularwobble * Smooth(0, 5, r) *
         perlinNoise(angularwobblefreq * rx, angularwobblefreq * ry, 0.1 * angularwobblefreq * rz);
    r += ringunevenness * perlinNoise(r, 0.31, 0.53);
    double inring = SmoothPulseTrain(0.1, 0.55, 0.7, 0.95, 1, r);
    inring = Mix(inring, 0.5, Smooth(0.2, 0.6, fw * ringfreq)); /* too fine to see: the average */

    /* the grain: two octaves of pores, stretched along the log */
    double gx = Px * grainfreq, gy = Py * grainfreq, gz = Pz * grainfreq * 0.05, dg = fw * grainfreq;
    double grain = 0;
    for (int i = 0; i < 2; i++, gx *= 2, gy *= 2, gz *= 2, dg *= 2) {
        const double valid = 1 - Smooth(0.2, 0.6, dg);
        if (valid <= 0)
            break;
        double g = valid * perlinNoise(gx, gy, gz);
        g *= 0.3 + 0.7 * inring;
        g = std::pow(std::max(0.0, std::min(1.0, 0.8 - g)), 2);
        g = grainy * Smooth(0.5, 1, g);
        if (i == 0)
            inring *= 1 - 0.4 * valid;
        grain = std::max(grain, g);
    }
    return Mix(inring, 1, grain);
}

/* WOOD: a plank of that oak under a finish. The plank's length runs
 * along the piece's own x, tilt degrees off the log's axis, with the pith
 * (ox, oz) wood units across and below the piece's origin: the pith below
 * and a few degrees of tilt cut ring after ring along the top, the nested
 * arches of a flat-sawn board. light and dark are the two tones; K wood
 * units per object unit (the rings 1/8 unit apart); grain the pores'
 * strength (1 is Gritz's); wobble how far the rings meander, in wood
 * units; fiber and fibersize the fibers that break up the rings' curves
 * (strength, and width in object units); eye and pixel (2 tan(fovy / 2)
 * over the image height) give a pixel's size at the hit, so detail finer
 * than a pixel fades instead of aliasing; specular and roughness are the
 * finish. The bound material supplies the rest. */
typedef struct {
    RtColor light, dark;
    RtFloat K, ox, oz, tilt, grain, wobble, fiber, fibersize;
    RtPoint eye;
    RtFloat pixel;
    RtColor specular;
    RtFloat roughness;
} WoodParams;

/* The oak at (x, y) across the log and z along it, in wood units, with
 * the wobble and the fibers of WoodParams: 0 light, 1 dark. */
inline double WoodAt(double x, double y, double z, const WoodParams *w, double fw)
{
    if (w->wobble > 0) {
        x += w->wobble * perlinNoise(0.5 * x + 3.1, 0.5 * y + 7.7, 0.1 * z);
        y += w->wobble * perlinNoise(0.5 * x + 11.3, 0.5 * y + 1.9, 0.1 * z + 5.3);
    }
    double wood = OakTexture(x, y, z, fw, w->grain);
    if (w->fiber > 0 && w->fibersize > 0) {
        const double f = 1 / (w->fibersize * w->K); /* fibers per wood unit, across the log */
        const double fade = 1 - Smooth(0.2, 0.6, fw * f);
        if (fade > 0) {
            const double n = perlinNoise(x * f, y * f, z * f * 0.07) +
                             0.5 * perlinNoise(2 * x * f + 9.1, 2 * y * f + 4.3, 2 * z * f * 0.07);
            wood = std::max(0.0, std::min(1.0, wood + w->fiber * fade * n));
        }
    }
    return wood;
}

inline void WOOD(RtShadeContext *c)
{
    const WoodParams *w = (const WoodParams *)c->params;
    const double dx = c->P[0] - w->eye[0], dy = c->P[1] - w->eye[1], dz = c->P[2] - w->eye[2];
    const double fw = w->K * std::sqrt(dx * dx + dy * dy + dz * dz) * w->pixel;
    const double tilt = w->tilt * 3.14159265358979323846 / 180, along = c->Po[0] * w->K;
    const double wood = WoodAt(c->Po[2] * w->K + w->ox, c->Po[1] * w->K + w->oz + along * std::sin(tilt),
                               along * std::cos(tilt), w, fw);
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = (RtFloat)Mix(w->light[i], w->dark[i], wood);
        c->m.specular[i] = w->specular[i];
    }
    c->m.roughness = w->roughness;
}

/* marble --------------------------------------------------------------- */

/* MARBLE: veins where a warped noise crosses zero. A noise field warps
 * the point (q = P / scale + warp x noise), a second noise is read at q,
 * and the albedo is the vein color where that noise is near zero (a
 * Gaussian of width `width`, so the veins are thin, tangled sheets cut by
 * the surface), the stone elsewhere, its tone clouded by a third noise; a
 * finer, fainter set of veins at twice the frequency. In object space
 * (c->Po), so the veins are carved into the stone. */
typedef struct {
    RtFloat scale;  /* object-space units per repeat of the noise: the vein spacing */
    RtFloat warp;   /* how far the noise bends the veins, in noise units */
    RtFloat width;  /* the veins' width, in noise units (0.05 is thin) */
    RtFloat cloud;  /* how much the stone's tone varies (0: none) */
    RtColor stone;  /* the albedo between the veins */
    RtColor vein;   /* the albedo at a vein's center */
} MarbleParams;

inline void MARBLE(RtShadeContext *c)
{
    const MarbleParams *p = (const MarbleParams *)c->params;
    const double x = c->Po[0] / p->scale, y = c->Po[1] / p->scale, z = c->Po[2] / p->scale;
    const double w1 = Fbm(x + 5.2, y + 1.3, z + 7.1, 4), w2 = Fbm(x - 3.7, y + 4.1, z + 2.2, 4);
    const double qx = x + p->warp * w1, qy = y + p->warp * w2, qz = z + p->warp * 0.5 * (w1 - w2);
    const double n1 = Fbm(qx, qy, qz, 5);
    const double n2 = Fbm(2.3 * qx + 9.1, 2.3 * qy + 3.3, 2.3 * qz + 6.7, 4);
    const double v1 = std::exp(-(n1 * n1) / (p->width * p->width));
    const double v2 = 0.5 * std::exp(-(n2 * n2) / (0.64 * p->width * p->width));
    const float t = (float)std::min(1.0, v1 + v2);
    const float tone = 1 - p->cloud * (float)(0.5 + 0.5 * Fbm(0.5 * qx + 2.4, 0.5 * qy + 8.8, 0.5 * qz + 1.9, 3));
    for (int i = 0; i < 3; i++)
        c->m.albedo[i] = p->stone[i] * tone + t * (p->vein[i] - p->stone[i] * tone);
}

#endif /* SCENEUTIL_H */
