/*
 * ECE/CS 285 – HW3 Hero Scene: Candy
 * A racing car made of sweets jumping over a chocolate-wafer road through a
 * candy landscape, a gummy cat at the wheel, 5.4 million triangles. The
 * colors come from the node trees' constants; this file adds its own
 * shaders (sky, gummy, ice cream, wafer, glitter, candy sticks, rubber), a
 * sun for the key and spot fills for the bounce light. The lamps glow by a
 * shader's emission, not as lights (GLOW). A thin lens 10 cm in radius
 * focused 24 m away, which whitted ignores; 256 spp, halton, seed 0 under
 * cook and kajiya.
 * Model: Candy Racing Car by Pokedstudio (the Blender 2.77 splash,
 * CC-BY-SA).
 *
 * Output: renders/<integrator>/3_HeroScenes/Candy.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"
#include "shaders.h"

#include <algorithm>
#include <cmath>
#include <cstring>
#include <vector>

/* shaders ------------------------------------------------------------ */

/* The sky: a backdrop quad far behind the scene whose emission runs from
 * cream to teal with the elevation of the ray that sees it (I points at
 * the surface, so I.y is the ray's climb), as a graded sky would. */
static void SKY(RtShadeContext *c)
{
    const float t = std::min(1.0f, std::max(0.0f, (c->I[1] - 0.16f) / 0.36f));
    const Rgb bottom = Color8(255, 232, 168), top = Color8(140, 205, 212);
    for (int i = 0; i < 3; i++) {
        c->m.emission[i] = bottom.v[i] + (top.v[i] - bottom.v[i]) * t;
        c->m.albedo[i] = 0;
        c->m.specular[i] = 0;
    }
}

/* The camera, for the haze. */
static const float EYE[3] = {24.5407f, -6.05775f, -8.10281f};

/* HAZE: beyond 50 m from the camera a surface fades toward the pale sky
 * by f = 0.5 (1 - exp(-(d - 50) / 150)): albedo and specular scaled by
 * 1 - f and the haze color added as emission. Every other shader here
 * ends by calling it, and a material that needs nothing else is bound
 * to it directly; the sky backdrop is the haze color already. */
static void HAZE(RtShadeContext *c)
{
    const float dx = c->P[0] - EYE[0], dy = c->P[1] - EYE[1], dz = c->P[2] - EYE[2];
    const float d = std::sqrt(dx * dx + dy * dy + dz * dz);
    const float f = 0.5f * (1 - std::exp(-std::max(0.0f, d - 50) / 150.0f));
    const Rgb haze = Color8(196, 222, 224);
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] *= 1 - f;
        c->m.specular[i] *= 1 - f;
        c->m.emission[i] = c->m.emission[i] * (1 - f) + haze.v[i] * f;
    }
}

/* The model's glass, hazed like everything else, at roughness 0: the
 * file's glass is clear, and the exporter wrote its roughness as Ns 5000,
 * which the OBJ loader reads as 0.14; under cook that would frost it a
 * little (glossy transmission, 2026-10-30). */
static void CLEAR_GLASS(RtShadeContext *c)
{
    HAZE(c);
    c->m.roughness = 0;
}

/* STEM: the candy sticks under the lollipop heads, honey-colored glass
 * (the bound material) with a faint grain in the body color, the albedo
 * times 1 + 0.12 noise, fine across the stick and long along it. */
static void STEM(RtShadeContext *c)
{
    const float n = (float)perlinNoise(c->P[0] * 6, c->P[1] * 0.6, c->P[2] * 6);
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = std::min(1.0f, c->m.albedo[i] * (1 + 0.12f * n));
    }
    HAZE(c);
}

/* GLOW: the model's lamps keep their emission (Ke) so they look lit, but
 * bound as the material's emission each OBJ group would become a light
 * (Whitted Illumination handout). The hook binds them with no emission
 * and this shader puts the glow back: seen directly and in mirrors, but
 * no light sample reaches it (under kajiya only the paths that happen to
 * hit a lamp pick up its glow). */
typedef struct {
    RtColor emission;
} GlowParams;

static void GLOW(RtShadeContext *c)
{
    const GlowParams *p = (const GlowParams *)c->params;
    for (int i = 0; i < 3; i++) {
        c->m.emission[i] = p->emission[i];
    }
    HAZE(c);
}

/* Tilt the shading normal by the gradient of a height field h(P): the
 * part of grad h across the surface is subtracted from N, scaled by
 * `strength`. The gradient is taken by central differences in world
 * space, so any scalar function of P will do. */
template <class H>
static void Bump(RtShadeContext *c, const H &h, float strength, float step)
{
    const float x = c->P[0], y = c->P[1], z = c->P[2];
    const float gx = (h(x + step, y, z) - h(x - step, y, z)) / (2 * step);
    const float gy = (h(x, y + step, z) - h(x, y - step, z)) / (2 * step);
    const float gz = (h(x, y, z + step) - h(x, y, z - step)) / (2 * step);
    const float ndotg = c->N[0] * gx + c->N[1] * gy + c->N[2] * gz;
    float n[3] = {c->N[0] - strength * (gx - c->N[0] * ndotg), c->N[1] - strength * (gy - c->N[1] * ndotg),
                  c->N[2] - strength * (gz - c->N[2] * ndotg)};
    const float len = std::sqrt(n[0] * n[0] + n[1] * n[1] + n[2] * n[2]);
    if (len > 0) {
        for (int i = 0; i < 3; i++) {
            c->N[i] = n[i] / len;
        }
    }
}

/* How much the view grazes the surface: 0 facing the camera, 1 at the
 * silhouette. N faces the ray and I points at the surface. */
static float Rim(const RtShadeContext *c)
{
    const float cosv = -(c->N[0] * c->I[0] + c->N[1] * c->I[1] + c->N[2] * c->I[2]);
    return 1 - std::min(1.0f, std::max(0.0f, cosv));
}

/* A repeatable hash of three integers to [0, 1): the coordinates and the
 * salt folded in with odd multipliers, then a full avalanche (two more
 * multiplies and shifts), so that neighboring cells share no structure.
 * (A one-multiply mix printed a fingerprint-like spiral in the glitter
 * about the point of a sphere that faces the camera, 2026-11-01.) */
static float Hash(int x, int y, int z, unsigned salt)
{
    unsigned h = (unsigned)x * 0x8da6b343u;
    h ^= (unsigned)y * 0xd8163841u;
    h ^= (unsigned)z * 0xcb1ab31fu;
    h ^= salt * 0x9e3779b9u;
    h ^= h >> 16;
    h *= 0x7feb352du;
    h ^= h >> 15;
    h *= 0x846ca68bu;
    h ^= h >> 16;
    return (float)(h & 0xffffffu) / 16777216.0f;
}

/* Glitter: the surface is tiled in object space into cells of `cell`
 * units, a fraction `density` of which hold a flake, a round dot (a ball
 * in the cell cut by the surface) whose normal is the smooth one tilted
 * by a Gaussian of standard deviation `tilt`, with a sharp highlight of
 * strength `flake` and roughness `flakeRoughness`. Because the tilts
 * cluster about the normal, the flakes facing the light are dense near
 * the mirror direction and thin out away from it: the highlight is a
 * core that dissolves into glints, as a sugar coat's does (a uniform
 * tilt gave a flat speckle with a hard edge; 2026-11-01). */
static void Glitter(RtShadeContext *c, float cell, float tilt, float density, float flake, float flakeRoughness = 0.4f)
{
    const int ix = (int)std::floor(c->Po[0] / cell), iy = (int)std::floor(c->Po[1] / cell), iz = (int)std::floor(c->Po[2] / cell);
    /* The flake is a ball of radius 0.25 to 0.45 cell anywhere in its
     * cell, cut by the surface: a round dot whose size varies with where
     * the surface crosses the ball. The point's own cell and its 26
     * neighbors are tried, since a ball may straddle a cell face; tested
     * against one cell alone, flakes were clipped at the faces and the
     * lattice printed rings and spirals where a curved surface grazes
     * its planes. */
    int fx = 0, fy = 0, fz = 0;
    bool hit = false;
    for (int dz = -1; dz <= 1 && !hit; dz++) {
        for (int dy = -1; dy <= 1 && !hit; dy++) {
            for (int dx = -1; dx <= 1 && !hit; dx++) {
                const int jx = ix + dx, jy = iy + dy, jz = iz + dz;
                if (Hash(jx, jy, jz, 7u) >= density) {
                    continue;
                }
                const float cx = ((float)jx + Hash(jx, jy, jz, 23u)) * cell;
                const float cy = ((float)jy + Hash(jx, jy, jz, 29u)) * cell;
                const float cz = ((float)jz + Hash(jx, jy, jz, 31u)) * cell;
                const float radius = (0.15f + 0.3f * Hash(jx, jy, jz, 37u)) * cell;
                const float ex = c->Po[0] - cx, ey = c->Po[1] - cy, ez = c->Po[2] - cz;
                if (ex * ex + ey * ey + ez * ez <= radius * radius) {
                    fx = jx; fy = jy; fz = jz;
                    hit = true;
                }
            }
        }
    }
    if (!hit) {
        return;
    }
    /* the tilt is Gaussian, standard deviation `tilt` per axis (Box-Muller
     * on two hashes per axis): most flakes lie near the surface normal and
     * few far from it, so the lit ones thin out away from the mirror
     * direction, a core that dissolves into glints */
    static const unsigned salts[6] = {0x27d4eb2fu, 0x165667b1u, 0x85ebca6bu, 0xc2b2ae35u, 0x9e3779b9u, 0x7feb352du};
    float g[3];
    for (int k = 0; k < 3; k++) {
        const float u1 = std::max(1e-6f, Hash(fx, fy, fz, salts[2 * k])), u2 = Hash(fx, fy, fz, salts[2 * k + 1]);
        g[k] = std::sqrt(-2 * std::log(u1)) * std::cos(2 * 3.14159265f * u2);
    }
    float n[3] = {c->N[0] + tilt * g[0], c->N[1] + tilt * g[1], c->N[2] + tilt * g[2]};
    const float len = std::sqrt(n[0] * n[0] + n[1] * n[1] + n[2] * n[2]);
    /* keep the flake on the outside: never past the geometric horizon */
    const float facing = (n[0] * c->Ng[0] + n[1] * c->Ng[1] + n[2] * c->Ng[2]) / len;
    if (facing > 0.1f) {
        for (int i = 0; i < 3; i++) {
            c->N[i] = n[i] / len;
        }
    }
    const float strength = flake * (0.4f + 0.6f * Hash(fx, fy, fz, 41u)); /* flakes differ in brightness too */
    for (int i = 0; i < 3; i++) {
        c->m.specular[i] = strength;
    }
    c->m.roughness = flakeRoughness;
}

/* A gummy with glitter: the bound albedo lightens a little toward the
 * silhouette, where light passing through a translucent sweet shows,
 * under a faint wide gloss (the highlight's soft core) and a dense
 * glitter (eight of ten 1.2 cm cells hold a flake, each smaller than a
 * pixel) whose lit flakes crowd the core and thin out from it: the
 * splash's sugar coat on a deep magenta. Both lobes are wider than the
 * 15 degree sun (roughness 0.45 and 0.5), so the sun is sampled on them
 * and a flake's brightness is the lobe's density at its angle, graded
 * further by its random strength and by the fraction of its pixel it
 * covers; the flakes are weak (0.25) so only those near the core clip.
 * (A flake lobe narrower than the sun made every lit flake a white
 * dot, the sun left to the lobe's ray, hit or miss; flakes wider than a
 * pixel landed as blocks under the box filter.) The gloss stays wide
 * too: tighter, it resolves the scene's ninety fill lights into pale
 * spots (seen at 0.22, 2026-11-01). */
static void GUMMY(RtShadeContext *c)
{
    const float r = Rim(c), glow = 0.2f * r * r;
    const float light[3] = {1.0f, 0.6f, 0.82f};
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = c->m.albedo[i] + (light[i] - c->m.albedo[i]) * glow;
        c->m.specular[i] = 0.05f;
    }
    c->m.roughness = 0.45f;
    Glitter(c, 0.012f, 0.10f, 0.8f, 0.25f, 0.5f);
    HAZE(c);
}

/* Ice cream: the scooped surface's dents as a bump (two octaves of noise
 * in object space), a faint mottle in the albedo, a lift toward the
 * silhouette, and a very soft highlight. */
static float ScoopHeight(float x, float y, float z)
{
    return 0.5f * (float)perlinNoise(x * 0.35, y * 0.35, z * 0.35) + 0.1f * (float)perlinNoise(x * 1.1, y * 1.1, z * 1.1);
}

/* The scenery scoops, with the dents. */
static void ICECREAM(RtShadeContext *c)
{
    Bump(c, ScoopHeight, 0.9f, 0.02f);
    const double n = perlinNoise(c->Po[0] * 1.5, c->Po[1] * 1.5, c->Po[2] * 1.5);
    const float mottle = 0.93f + 0.07f * (float)n, r = Rim(c), lift = 1 + 0.2f * r * r;
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = std::min(1.0f, c->m.albedo[i] * mottle * lift);
        c->m.specular[i] = 0; /* no gloss: ice cream is matte, and under cook a lobe here only
                                  mirrored the scoops around into the shadow sides (2026-11-01) */
    }
    c->m.roughness = 0.67f;
    HAZE(c);
}

/* A sandwich cookie's face: the embossed pattern as a bump, concentric
 * rings and radial scallops about the cookie's axis within its radius.
 * The shader takes the center and axis, and whether to measure in P or
 * Po (the cookie by the car is in world space, the flying one is not). */
struct CookieParams {
    RtPoint center;
    RtVector axis;
    RtInt objectSpace;
};

static void COOKIE(RtShadeContext *c)
{
    const CookieParams *p = (const CookieParams *)c->params;
    const float *pt = p->objectSpace ? c->Po : c->P;
    const float dx = pt[0] - p->center[0], dy = pt[1] - p->center[1], dz = pt[2] - p->center[2];
    const float ax = p->axis[0], ay = p->axis[1], az = p->axis[2];
    const float along = dx * ax + dy * ay + dz * az;
    const float rx = dx - ax * along, ry = dy - ay * along, rz = dz - az * along;
    const float r = std::sqrt(rx * rx + ry * ry + rz * rz);
    if (r > 1.2f) {
        return;
    }
    /* two tangents in the face: t1 = up projected into the face, t2 = axis x t1 */
    const float t1x = -ax * ay, t1y = 1 - ay * ay, t1z = -az * ay;
    const float t2x = ay * t1z - az * t1y, t2y = az * t1x - ax * t1z, t2z = ax * t1y - ay * t1x;
    const float u0 = rx * t1x + ry * t1y + rz * t1z, v0 = rx * t2x + ry * t2y + rz * t2z;
    (void)u0;
    (void)v0;
    /* the height field in world coordinates, so Bump can difference it */
    const auto h = [&](float x, float y, float z) {
        const float qx = (p->objectSpace ? x - c->P[0] + c->Po[0] : x) - p->center[0];
        const float qy = (p->objectSpace ? y - c->P[1] + c->Po[1] : y) - p->center[1];
        const float qz = (p->objectSpace ? z - c->P[2] + c->Po[2] : z) - p->center[2];
        const float a = qx * ax + qy * ay + qz * az;
        const float sx = qx - ax * a, sy = qy - ay * a, sz = qz - az * a;
        const float rr = std::sqrt(sx * sx + sy * sy + sz * sz);
        const float u = sx * t1x + sy * t1y + sz * t1z, v = sx * t2x + sy * t2y + sz * t2z;
        const float theta = std::atan2(v, u);
        const float rings = std::sin(rr * 34), scallops = std::sin(theta * 16) * std::min(1.0f, rr * 2);
        return 0.012f * (rings + 0.8f * scallops);
    };
    Bump(c, h, 2.0f, 0.004f);
}

/* The boosters' scoops: the same ice cream, smooth. */
static void ICECREAM_SMOOTH(RtShadeContext *c)
{
    const double n = perlinNoise(c->Po[0] * 1.5, c->Po[1] * 1.5, c->Po[2] * 1.5);
    const float mottle = 0.93f + 0.07f * (float)n, r = Rim(c), lift = 1 + 0.2f * r * r;
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = std::min(1.0f, c->m.albedo[i] * mottle * lift);
        c->m.specular[i] = 0; /* no gloss: ice cream is matte, and under cook a lobe here only
                                  mirrored the scoops around into the shadow sides (2026-11-01) */
    }
    c->m.roughness = 0.67f;
    HAZE(c);
}

/* The chocolate wafer: a bar of square cells with rounded edges, a
 * narrow groove between them, and crumbs. The cells are flat, so the
 * normal only tilts at the grooves and the edges come out crisp. */
static float Smoothstep(float a, float b, float x)
{
    const float t = std::min(1.0f, std::max(0.0f, (x - a) / (b - a)));
    return t * t * (3 - 2 * t);
}

static float WaferHeight(float x, float y, float z)
{
    const float cell = 0.7f;
    /* cell coordinates in a grid turned 30 degrees about y, roughly along the road */
    const float u = 0.866f * x - 0.5f * z, w = 0.5f * x + 0.866f * z;
    const float fu = std::fabs(u / cell - std::floor(u / cell) - 0.5f), fw = std::fabs(w / cell - std::floor(w / cell) - 0.5f);
    const float edge = std::max(fu, fw); /* 0 at a cell's center, 0.5 at its border */
    return -0.3f * Smoothstep(0.38f, 0.47f, edge) + 0.02f * (float)perlinNoise(x * 9, y * 9, z * 9);
}

static float CrumbHeight(float x, float y, float z)
{
    return 0.03f * (float)perlinNoise(x * 9, y * 9, z * 9);
}

static void WAFER(RtShadeContext *c)
{
    /* the cells are pressed into the top of the bar; the sides are plain */
    if (c->Ng[1] > 0.45f) {
        Bump(c, WaferHeight, 2.5f, 0.006f);
    } else {
        Bump(c, CrumbHeight, 1.5f, 0.006f);
    }
    const float tone = 0.92f + 0.08f * (float)perlinNoise(c->P[0] * 2, c->P[1] * 2, c->P[2] * 2);
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] *= tone;
    }
    HAZE(c);
}

/* The grill and bumper: a sugar-crust speckle in the albedo. */

static void SPECKLE(RtShadeContext *c)
{
    const float s = 0.78f + 0.3f * (float)perlinNoise(c->P[0] * 40 + 3, c->P[1] * 40, c->P[2] * 40);
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = std::min(1.0f, c->m.albedo[i] * s);
    }
    HAZE(c);
}

/* The car's candy paint: a very faint mottle in the color, no relief,
 * and a metallic-flake glitter, finer and fainter than the gummy's, over
 * the bound paint gloss. */
static void FROSTED(RtShadeContext *c)
{
    const float s = 0.96f + 0.05f * (float)perlinNoise(c->P[0] * 40 + 3, c->P[1] * 40, c->P[2] * 40);
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = std::min(1.0f, c->m.albedo[i] * s);
    }
    Glitter(c, 0.007f, 0.2f, 1.0f, 0.7f); /* a dot in every cell */
    HAZE(c);
}

/* RUBBER: the tires. The file's base colors at 1.25 (the splash's are
 * deeper than the MTL's; the pink keeps the MTL's magenta), the MTL's
 * wide gloss at F0 0.02, a faint bump of two noise scales and a fine
 * grain, a cell about a pixel across darkened at random (picked
 * from the sweeps, 2026-11-04, against the file's Cycles render of the
 * sunlit tires; the file's satin gloss, F0 0.10 at roughness 0.45,
 * prints the fill spots as streaks along the treads). */
typedef struct {
    float color[3];
} RubberParams;

static const RubberParams RUBBER_COLORS[4] = {
    {{0.72f, 0.58f, 0.014f}}, /* black-rubber.tyre: yellow */
    {{0.72f, 0.021f, 0.01f}}, /* .001: red */
    {{0.93f, 0.10f, 0.50f}},  /* .002: pink (the file's is purple, 0.55 0.036 0.72) */
    {{0.08f, 0.72f, 0.043f}}, /* .003: green */
};

static void RUBBER(RtShadeContext *c)
{
    const RubberParams *p = (const RubberParams *)c->params;
    const auto height = [](float x, float y, float z) {
        return 0.0005f * (float)perlinNoise(x * 60.0, y * 60.0, z * 60.0) +
               0.0002f * (float)perlinNoise(x * 150.0, y * 150.0, z * 150.0);
    };
    Bump(c, height, 1.0f, 0.001f);
    const int cx = (int)std::floor(c->P[0] / 0.012f), cy = (int)std::floor(c->P[1] / 0.012f),
              cz = (int)std::floor(c->P[2] / 0.012f);
    const float grain = 1 - 0.5f * Hash(cx, cy, cz, 7u);
    for (int i = 0; i < 3; i++) {
        c->m.albedo[i] = p->color[i] * grain;
        c->m.specular[i] = 0.02f;
    }
    c->m.roughness = 0.62f;
    HAZE(c);
}

/* What the hook needs from the scene: the cookie's parameters, for the
 * mesh that holds a cookie, and the mesh being drawn, whose MTL materials
 * say which ones glow. Either may be NULL. */
typedef struct {
    const CookieParams *cookie;
    const ObjMesh *mesh;
} CandyHook;

/* Which OBJ materials get a shader of their own. */
static void CandyMaterials(const char *material, void *user)
{
    const CandyHook *hook = (const CandyHook *)user;
    /* The candy canes: the MTL's specular 0.12 mirrored the scoops and the
     * sky along them; 0.04 keeps the sun's highlight at full brightness
     * (its lobe's peak density times the sun is 23 times a reflection of
     * the scoops) with no visible reflection (picked from a sweep,
     * 2026-11-01). The lobe is the MTL's, roughness 0.47. */
    if (std::strcmp(material, "red") == 0 || std::strcmp(material, "white") == 0 || std::strcmp(material, "red.002") == 0
        || std::strcmp(material, "white.001") == 0) {
        for (int i = 0; hook && hook->mesh && i < hook->mesh->nmaterials; i++) {
            if (std::strcmp(hook->mesh->materials[i].name, material) == 0) {
                RtMaterial m = ObjMaterialToRt(hook->mesh->materials[i]);
                m.specular[0] = m.specular[1] = m.specular[2] = 0.04f;
                rtMaterial(&m);
            }
        }
        rtSurface(HAZE, NULL, 0);
        return;
    }
    /* The tires: RUBBER, one color each (the wheel's index from the
     * material's suffix: none, .001, .002, .003). */
    if (std::strncmp(material, "black-rubber.tyre", 17) == 0) {
        const int wheel = material[17] == '.' ? std::min(3, std::max(0, material[20] - '0')) : 0;
        rtSurface(RUBBER, &RUBBER_COLORS[wheel], sizeof(RubberParams));
        return;
    }
    /* The cookies' biscuit (MTL specular 0.03): its lobe is wide
     * (roughness 0.59), so a highlight is only a few times a reflection;
     * at 0.011 a hint of gloss stays and the reflected scoops go
     * (picked from a sweep, 2026-11-01). The near cookie keeps
     * its COOKIE embossing. */
    if (std::strcmp(material, "biscuit") == 0 || std::strcmp(material, "biscuit.001") == 0) {
        for (int i = 0; hook && hook->mesh && i < hook->mesh->nmaterials; i++) {
            if (std::strcmp(hook->mesh->materials[i].name, material) == 0) {
                RtMaterial m = ObjMaterialToRt(hook->mesh->materials[i]);
                m.specular[0] = m.specular[1] = m.specular[2] = 0.011f;
                rtMaterial(&m);
            }
        }
        if (std::strcmp(material, "biscuit") == 0 && hook && hook->cookie) {
            rtSurface(COOKIE, hook->cookie, sizeof(CookieParams));
            return;
        }
        rtSurface(HAZE, NULL, 0);
        return;
    }
    static const char *const scoops[] = {"Material.030", "Material.029", "Material.038", "Material.025", "Material.026",
                                         "blue-icecream", ""};
    if (std::strcmp(material, "booster_scoop") == 0 || std::strcmp(material, "default") == 0) {
        rtSurface(ICECREAM_SMOOTH, NULL, 0);
        return;
    }
    if (std::strcmp(material, "blue.004") == 0) {
        rtSurface(GUMMY, NULL, 0);
        return;
    }
    if (std::strcmp(material, "biscuit.002") == 0) {
        rtSurface(WAFER, NULL, 0);
        return;
    }
    if (std::strcmp(material, "gloss-metal-straight.007") == 0) {
        rtSurface(SPECKLE, NULL, 0);
        return;
    }
    if (std::strcmp(material, "blue") == 0) {
        rtSurface(FROSTED, NULL, 0);
        return;
    }
    if (std::strncmp(material, "glass", 5) == 0) {
        rtSurface(CLEAR_GLASS, NULL, 0);
        return;
    }
    /* The yellow balloon's skin: the MTL's gloss at roughness 0.38, not
     * its 0.18, which resolved the fill lights into pale dots across the
     * ball under cook, as a tight gloss did on the cat (picked
     * from a sweep, 2026-11-06). */
    if (std::strcmp(material, "blue.001") == 0) {
        for (int i = 0; hook && hook->mesh && i < hook->mesh->nmaterials; i++) {
            if (std::strcmp(hook->mesh->materials[i].name, material) == 0) {
                RtMaterial m = ObjMaterialToRt(hook->mesh->materials[i]);
                m.roughness = 0.38f;
                rtMaterial(&m);
            }
        }
        rtSurface(HAZE, NULL, 0);
        return;
    }
    /* The cat's nose: a soft sheen and no mirror (the MTL's Ks 0.2 at
     * Ns 70 read as a chrome blob). */
    /* The cat's eyes: dark, a soft highlight and little reflection (the
     * MTL's specular 0.3 mirrored the scene in them). */
    if (std::strcmp(material, "Material.011") == 0) {
        const RtMaterial eye = {{0.01f, 0.01f, 0.01f}, {0.08f, 0.08f, 0.08f}, 0.5f, 0, 0, 1.5f, {0, 0, 0}};
        rtMaterial(&eye);
        rtSurface(HAZE, NULL, 0);
        return;
    }
    if (std::strcmp(material, "nose") == 0) {
        const RtMaterial nose = {{0.11f, 0.055f, 0.03f}, {0.03f, 0.03f, 0.03f}, 0.55f, 0, 0, 1.5f, {0, 0, 0}};
        rtMaterial(&nose);
        rtSurface(HAZE, NULL, 0);
        return;
    }
    /* The candy sticks: the file's translucent sweet, which the exporter
     * read as dark wood; honey in a straw: clear glass passing 95% of the
     * light, tinted by the body color (the albedo tints what a dielectric
     * transmits, HW3's rule) with STEM's grain in it, at an ior of 1.15
     * so the scene shows through the rod instead of being bent into a
     * band (at 1.5 a rod is a cylindrical lens). */
    if (std::strcmp(material, "wood-direction.003") == 0) {
        const RtMaterial stem = {{0.98f, 0.70f, 0.28f}, {0.04f, 0.04f, 0.04f}, 0, 0, 0.95f, 1.15f, {0, 0, 0}};
        rtMaterial(&stem);
        rtSurface(STEM, NULL, 0);
        return;
    }
    if (std::strncmp(material, "gloss-metal-straight", 20) == 0) {
        /* The ridged booster tubes: brushed steel, specular 0.35 (the
         * reflectance head on) over a dark base, roughness 0.45 (alpha
         * 0.2): glossy, not mirror-bright (0.55 and 0.28 read as chrome).
         * cook blurs its reflections through the lobe; under kajiya
         * metallic 1 drops the diffuse under the lobe, a conductor of
         * reflectance 0.35 with a wide GGX lobe. */
        const RtMaterial steel = {{0.2f, 0.2f, 0.2f}, {0.35f, 0.35f, 0.36f}, 0.45f, 1, 0, 1.5f, {0, 0, 0}};
        rtMaterial(&steel);
        rtSurface(HAZE, NULL, 0);
        return;
    }
    for (int i = 0; scoops[i][0]; i++) {
        if (std::strcmp(material, scoops[i]) == 0) {
            rtSurface(ICECREAM, NULL, 0);
            return;
        }
    }
    for (int i = 0; hook && hook->mesh && i < hook->mesh->nmaterials; i++) {
        const ObjMaterial &om = hook->mesh->materials[i];
        if (std::strcmp(material, om.name) != 0 || !(om.Ke[0] > 0 || om.Ke[1] > 0 || om.Ke[2] > 0)) {
            continue;
        }
        RtMaterial m = ObjMaterialToRt(om); /* what ObjDraw bound, less the emission */
        GlowParams glow;
        for (int k = 0; k < 3; k++) {
            glow.emission[k] = m.emission[k];
            m.emission[k] = 0;
        }
        rtMaterial(&m);
        rtSurface(GLOW, &glow, sizeof glow);
        return;
    }
    rtSurface(HAZE, NULL, 0);
}

/* scene -------------------------------------------------------------- */

/* A dome of weak spot lights: `count` of them spread evenly over the
 * band of elevations [lo, hi] (fractions of the vertical, -1 to 1) on a
 * sphere of radius r about `center`, each aimed at the center with a wide
 * cone, together giving a facing surface the irradiance `E`. */
static void Dome(const Vec3 &center, float r, float lo, float hi, int count, const RtColor E, unsigned seed)
{
    /* I per light: count lights at distance r, average cosine 1/2 */
    const float I = E[0] > 0 || E[1] > 0 || E[2] > 0 ? 2 * r * r / (float)count : 0;
    const float golden = 2.39996323f;
    for (int i = 0; i < count; i++) {
        const float y = lo + (hi - lo) * ((i + 0.5f) / count);
        const float ring = std::sqrt(std::max(0.0f, 1 - y * y));
        const float a = golden * i + 0.01f * (float)(seed % 97);
        const Vec3 d(ring * std::cos(a), y, ring * std::sin(a));
        const Vec3 p = center + d * r;
        RtPoint pos = {p.x, p.y, p.z};
        RtVector dir = {-d.x, -d.y, -d.z};
        RtColor Ic = {E[0] * I, E[1] * I, E[2] * I};
        rtSpotLight(pos, dir, Ic, 75, 25, 0);
    }
}

/* The lights: the sun is the key. Everything else stands in for the
 * bounce light a path tracer would give for free: a dome of weak spots
 * for the sky, a low band for the ground bounce, fills under the car and
 * grazing spots on the cookie beside it. */
static void CandyLights(void)
{
    /* The sun: the file's, 15.5 degrees across (a soft, wide sun), a disc
     * 1 km out over the car. */
    RtColor sunE = {3.6f, 3.5f, 3.1f};
    SunDisc(Vec3(-0.635f, -0.763f, -0.123f), sunE, 15.5f, Vec3(2.0f, 1.0f, 0.9f), 1000.0f);

    /* the sky: 40 lights over the upper hemisphere, 70 m out, E 0.9 */
    const RtColor skyE = {0.7f, 0.85f, 0.92f};
    Dome(Vec3(2, 2, 0), 70, 0.3f, 0.95f, 40, skyE, 1); /* from elevation 17 degrees up: little sky under the car */
    /* the ground bounce: 14 lights low around the scene, warm, E 0.35 */
    const RtColor groundE = {0.42f, 0.33f, 0.22f};
    Dome(Vec3(2, 2, 0), 70, -0.45f, -0.05f, 14, groundE, 2);

    /* the balloon's face (the ball at (-1.9, 4.7, -1.4), radius 0.85): a
     * fill a quarter of the way out along the line from the ball to the
     * camera (the one line sure to be clear: the booster's scoop hides
     * the ball from most other places), 7.3 away, a third of the sun's
     * irradiance at the face, in a cone just wide enough for the ball (no
     * reach limit: the cone keeps it local) */
    RtPoint balloonPos = {4.7f, 2.0f, -3.1f};
    RtVector balloonDir = {-6.6f, 2.7f, 1.7f};
    RtColor balloonI = {75, 71, 64};
    rtSpotLight(balloonPos, balloonDir, balloonI, 9, 4, 0);

    /* the cookie beside the car (center (-1.5, 2.1, -3.7), radius 1, its
     * face toward (0.77, 0, -0.64)): a small light just above its rim and
     * a hair in front of its face, shining straight down the face, so the
     * light grazes it and every ridge throws a shadow */
    /* both 0.35 in front of the face (along its axis (0.77, 0, -0.64)) and
     * tilted a little toward it, so they see the face and graze it */
    RtPoint cookieTopPos = {-1.17f, 3.45f, -3.97f}, cookieRightPos = {-0.31f, 2.1f, -2.82f};
    RtVector cookieTopDir = {-0.19f, -1, 0.16f}, cookieRightDir = {-0.83f, 0, -0.61f};
    /* about the sun's irradiance at the face (I / d^2 with d about 1.4),
     * so the chocolate shades without clipping to white */
    RtColor cookieTopI = {7.5f, 7, 6}, cookieRightI = {5, 4.6f, 4};
    rtSpotLight(cookieTopPos, cookieTopDir, cookieTopI, 34, 12, 2.7f);
    rtSpotLight(cookieRightPos, cookieRightDir, cookieRightI, 34, 12, 2.7f);
    /* and, since the car keeps the sun off it, a frontal fill 2.2 out along
     * its axis at the sun's irradiance, so the chocolate reads as it does
     * on the sunlit cookie above */
    RtPoint cookieFrontPos = {0.19f, 2.1f, -5.1f};
    RtVector cookieFrontDir = {-0.77f, 0, 0.64f};
    RtColor cookieFrontI = {14, 13, 11.5f};
    rtSpotLight(cookieFrontPos, cookieFrontDir, cookieFrontI, 30, 12, 4);

    /* under the car (the underside spans x -0.3..4.4, z -1.7..3.5 at
     * y 0.2): twenty-four weak fills in two rings just below the
     * underside's level and out to its sides, each aimed across at a
     * point on the far edge rather than up, with a short reach, so the
     * parts catch light from the sides and shadow one another, and no one
     * light shows as a beam */
    for (int i = 0; i < 24; i++) {
        const float a = 6.2831853f * i / 24;
        const float r = i % 2 ? 4.2f : 3.4f, y = i % 2 ? -1.1f : -0.5f;
        const Vec3 p(2.0f + r * std::cos(a), y, 0.9f + 0.8f * r * std::sin(a));
        const Vec3 target(2.0f - 1.5f * std::cos(a), -0.1f, 0.9f - 1.2f * std::sin(a));
        const Vec3 d = normalize(target - p);
        RtPoint pos = {p.x, p.y, p.z};
        RtVector dir = {d.x, d.y, d.z};
        RtColor I = {0.9f, 0.55f, 0.3f}; /* the warm bounce off the sweets around */
        rtSpotLight(pos, dir, I, 30, 14, 6.5f);
    }
    /* kickers: six small bright spots low on the camera's side (the
     * splash has hard bright edges on the shaft, the axle arms and the
     * springs, against a dark underside), each aimed at one part */
    {
        const Vec3 targets[6] = {Vec3(1.1f, -0.2f, 1.0f), Vec3(3.1f, -0.2f, 1.0f), Vec3(0.2f, 0.1f, -0.7f),
                                 Vec3(4.3f, 0.2f, -0.7f), Vec3(4.4f, -0.4f, 2.1f), Vec3(2.0f, -0.6f, 2.7f)};
        for (int i = 0; i < 6; i++) {
            const Vec3 p = targets[i] + Vec3(2.6f, -1.6f, 1.4f);
            const Vec3 d = normalize(targets[i] - p);
            RtPoint pos = {p.x, p.y, p.z};
            RtVector dir = {d.x, d.y, d.z};
            RtColor I = {8.0f, 5.6f, 3.4f};
            rtSpotLight(pos, dir, I, 14, 7, 5.5f);
        }
    }
    /* and the warm bounce off the wafer and the ice cream below: a 4 x 4
     * grid of weak spots 2.5 m under the car aimed straight up, wide, so
     * the undersides of the axles and the shaft are not black */
    for (int i = 0; i < 4; i++) {
        for (int j = 0; j < 4; j++) {
            RtPoint pos = {-0.5f + 1.7f * i, -2.3f, -1.5f + 1.6f * j};
            RtVector up = {0, 1, 0};
            RtColor I = {3.0f, 1.7f, 0.7f}; /* the wafer's orange */
            rtSpotLight(pos, up, I, 55, 20, 4.5f);
        }
    }
}

void Candy(void)
{
    /* The camera: a 35 mm lens, 26.7 degrees tall. */
    RtPoint eye = {EYE[0], EYE[1], EYE[2]}, lookat = {23.65881f, -5.73568f, -7.75853f};
    RtVector up = {0.22355f, 0.92863f, -0.29611f};
    RtColor ka = {0, 0, 0}; /* no ambient: every lit surface is lit by a light */
    RtMaterial sky = MATTE(0, 0, 0);

    rtBegin();
        rtFormat(1920, 1080);
        rtDisplay("3_HeroScenes/Candy");
        rtPixelSamples(256); /* cook and kajiya; whitted renders one sample */
        rtSampler("halton");
        rtCamera(eye, lookat, up, 26.73f);
        /* The splash's depth of field: a lens 10 cm in radius focused on
         * the gummy cat, 24 m away, so the car is sharp and the candy in
         * front of it and the far scoops blur. whitted ignores the lens. */
        rtLens(0.1f, 24.0f);
        rtAmbient(ka);
        rtBackground(Color8(140, 205, 212)); /* the top of the sky, for rays past the backdrop */
        rtWorldBegin(0);
            CandyLights();

            /* The backdrop: a quad 600 m behind the scene facing the
             * camera, with the sky gradient as its emission. */
            rtMaterial(&sky);
            rtSurface(SKY, NULL, 0);
            Quad(Vec3(-600, -400, 900), Vec3(0, 1000, 0), Vec3(0, 0, -1800));
            rtSurface(NULL, NULL, 0);

            /* the cookie beside the car: center (-1.5, 2.1, -3.7), its axis
             * toward the camera (0.77, 0, -0.64), measured in world space */
            CookieParams nearCookie = {{-1.5f, 2.1f, -3.7f}, {0.77f, 0, -0.64f}, 0};
            ObjMesh *world = obj_load("models/candy/candy.mesh");
            CandyHook worldHook = {&nearCookie, world};
            ObjDraw(world, 0, CandyMaterials, &worldHook);
            obj_free(world);
            ObjDrawInstances("models/candy/instances.txt", 0, CandyMaterials, NULL);

            /* Glass lenses over the headlights. The file already covers each
             * lamp with a flat glass slab; a squeezed glass sphere, a little
             * wider than the lamp and a tenth as thick (a thin dome: a glass
             * cover, not a magnifier), sits 0.03 in front of that slab's
             * plane (a lens overlapping another glass surface sends rays
             * through three refractions and the depth limit, which shows as
             * speckle), so the glow shows through the dome and the sky
             * reflects in it. Six lamps on the arms (slab fronts at x 0.862
             * facing +x toward the camera, or -0.965 facing -x) and the two
             * in the car's face (slab front at x 5.591, facing +x).
             * Entries: slab front x, center y z, facing, lens radius. */
            {
                const float lamps[8][5] = {{0.862f, 5.565f, 3.068f, 1, 0.4f}, {0.862f, 5.582f, -1.049f, 1, 0.4f},
                                           {0.865f, 6.217f, 0.161f, 1, 0.4f}, {0.759f, 7.257f, 0.893f, 1, 0.4f},
                                           {-0.965f, 5.565f, 1.051f, -1, 0.4f}, {-0.958f, 5.565f, -0.301f, -1, 0.4f},
                                           {5.591f, 0.735f, -0.76f, 1, 0.29f}, {5.591f, 0.735f, 2.794f, 1, 0.29f}};
                RtMaterial lens = GLASS;
                rtMaterial(&lens);
                for (int i = 0; i < 8; i++) {
                    const float r = lamps[i][4], half = 0.1f * r;
                    rtTransformBegin();
                        rtTranslate(lamps[i][0] + (half + 0.03f) * lamps[i][3], lamps[i][1], lamps[i][2]);
                        rtScale(half, r, r);
                        rtSphere(1);
                    rtTransformEnd();
                }
            }

            /* A second sandwich cookie flying above the left booster, as in
             * the splash: the one beside the car (halves, ridges and the
             * cream bulge), exported on its own and placed here. */
            ObjMesh *cookie = obj_load("models/candy/cookie/cookie.mesh");
            rtTransformBegin();
                rtTranslate(2.2f, 8.9f, 4.6f); /* back and up: its sun shadow falls behind the cat */
                /* the model's face already points at the camera; tipped back
                 * about the screen's horizontal axis so we look at it from a
                 * little below, with the cream edge along the bottom, and
                 * swung a little aside, as in the splash */
                rtRotate(-35, -0.34f, 0, -0.88f);
                rtRotate(12, 0, 1, 0);
                rtScale(1.65f, 1.65f, 1.65f);
                CookieParams flyingCookie = {{0, 0, 0}, {0.77f, 0, -0.64f}, 1}; /* the model is centered on its origin */
                CandyHook cookieHook = {&flyingCookie, cookie};
                ObjDraw(cookie, 0, CandyMaterials, &cookieHook);
            rtTransformEnd();
            obj_free(cookie);
        rtWorldEnd();
    rtEnd();
}
