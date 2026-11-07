/*
 * ECE/CS 285 – HW2 Hero Scene: Barbershop
 * The Agent 327 barbershop from its back wall: the counter and its
 * mirrors on the left, two chairs under the chandeliers, the picture
 * wall on the right, the window and the door at the far end; glass
 * cabinets, chrome, 14 million triangles, in daylight, the shop's lamps
 * off: a sun outside the window shining in between the blinds' slats
 * and the white world behind them, in full sun, lighting the room.
 * BarbershopSunOnly is that light alone, nothing hidden, no ambient;
 * Barbershop adds hidden spot fills for the bounce light whitted lacks.
 * 1920 x 1080; 2048 spp, halton, seed 0 under cook and kajiya.
 * Model: the Agent 327 Barbershop by the Blender Animation Studio
 * (blender.org demo files, CC-BY).
 *
 * Output: renders/<integrator>/3_HeroScenes/Barbershop.png and
 *         BarbershopSunOnly.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

#include <cmath>
#include <cstring>

/* Everything scaled by this. */
static const float EXPOSURE = 1.0f;

/* The shop's lamps, the practicals and their bulbs, times this: off,
 * the daylight alone lights the room; 1 is the file's own strengths. The
 * number after the scene's name on the command line overrides it, for
 * tests (gauchoRT Barbershop 1). */
static const float LAMP_SCALE_DEFAULT = 0.0f;
static float lampScale = LAMP_SCALE_DEFAULT;
static float sunScale = 1, backdropScale = 1, blindsScale = 1; /* the sun, the white world behind the blinds and the
                                            blinds' own glow: the second, third and fourth scene arguments, for
                                            tests */

/* panes --------------------------------------------------------------- */

/* The window, the door, the lamps' glass, the cabinets, the picture glass
 * and the bottles are, in the file, transparent to shadow rays (a Light
 * Path trick): what is behind or inside them is lit as if they were not
 * there. Here these materials pass shadow rays. */
static const char *const PANES[] = {
    "window_glass_barbershop", "door_glass", "lamp_glass", "counter_glass", "dresser_glass",
    "picture_frame_glass", "clock_wall_glass", "Cleanser_Liquid", "Disinfectant_Liquid",
    "bottle_liquid", "White_Plastic", "plastic_wrap", "Smudge_Glass", "White_Rough_Glass",
    "lampshade_ceiling", /* the chandeliers' shades: the bulb inside lights the ceiling through them, as a thin
                            shade does, instead of printing their pattern on it */
}; /* not the cobwebs: their holes are an alpha map, which ObjDraw's cutout keeps */

/* The camera stands 0.3 m behind the back wall's panel, inside the wall,
 * with the curtain beside it; the file clips everything nearer than 1.1 m
 * (the camera's clip start), so here every material is a cutout for a
 * camera ray within that depth. */
static const float EYE[3] = {2.968f, 1.545f, -9.955f}, FORWARD[3] = {-0.283f, -0.052f, 0.958f};
static bool nearTheCamera(const RtShadeContext *c)
{
    const float depth = (c->P[0] - EYE[0]) * FORWARD[0] + (c->P[1] - EYE[1]) * FORWARD[1] + (c->P[2] - EYE[2]) * FORWARD[2];
    return c->depth == 0 && depth < 1.1f;
}

static RtInt NEAR_CLIP(RtShadeContext *c) { return nearTheCamera(c) ? 0 : 1; }
static RtInt PANE(RtShadeContext *c) { return c->depth < 0 || nearTheCamera(c) ? 0 : 1; }

/* ObjDraw's alpha cutout (map_d at a half) with the near clip. */
typedef struct { RtInt map; } AlphaParams;
static RtInt ALPHA_NEAR(RtShadeContext *c)
{
    const AlphaParams *p = (const AlphaParams *)c->params;
    if (nearTheCamera(c))
        return 0;
    RtFloat rgba[4];
    rtTexture(p->map, c->u, c->v, rgba);
    return rgba[0] >= 0.5f ? 1 : 0;
}

/* The cobwebs under the ceiling: gauze in the file, here an alpha map
 * at a half, which would print every sheet's shape on the ceiling. They
 * pass shadow rays; the camera still sees them. */
static RtInt COBWEB(RtShadeContext *c) { return c->depth < 0 ? 0 : ALPHA_NEAR(c); }

/* The blinds: white plastic the daylight lights from behind, the
 * brightest thing in the room's view; here they glow at the radiance
 * Cycles shows them at, which also makes them a light (its stand-in a
 * point at their center under whitted). The white world behind them,
 * in full sun, is what lights the room: the file shows the camera a card
 * outside the window ("Material", the street, at radiance (1, 0.92,
 * 0.7)); here that card is hidden and the window's opening gets a grid
 * of 32 emitting cells just outside the pane, at a thousand times
 * that radiance, letting shadow rays by so the sun comes in past them.
 * Under kajiya the cells are the card, sampled as 32 lights. Under
 * whitted each cell is a point light at its center: the four rows see a
 * point in the room through different slat gaps, so together they give
 * the gaps' share of the light where one card's stand-in gave all of it
 * or nothing, and the eight columns spread the shadows the hanging
 * fixtures throw back onto the ceiling, which the window's width washes
 * out under kajiya. A thousand times is bright enough to light the room
 * by bounce alone under kajiya, which is what the scene sets against
 * whitted. */
static const float BLINDS_RADIANCE = 1.5f;
static const RtColor BACKDROP_RADIANCE = {1000.0f, 919.0f, 700.0f};
static const float WINDOW_X0 = 1.70f, WINDOW_X1 = 3.21f, WINDOW_Y0 = 1.24f, WINDOW_Y1 = 2.04f; /* the opening */
static const float WINDOW_Z = -0.70f; /* just outside the pane, which is the front wall at z = -0.78 */
static const int WINDOW_ROWS = 4, WINDOW_COLS = 8;

static RtInt HIDDEN(RtShadeContext *) { return 0; }

typedef struct {
    const ObjMesh *mesh;
} BarbershopHook;

static void BarbershopMaterials(const char *material, void *user)
{
    const BarbershopHook *hook = (const BarbershopHook *)user;
    const ObjMesh *mesh = hook->mesh;
    const ObjMaterial *om = NULL;
    for (int i = 0; mesh && i < mesh->nmaterials; i++)
        if (std::strcmp(mesh->materials[i].name, material) == 0)
            om = &mesh->materials[i];
    for (size_t i = 0; i < sizeof PANES / sizeof PANES[0]; i++)
        if (std::strncmp(material, PANES[i], std::strlen(PANES[i])) == 0) {
            rtCutout(PANE, NULL, 0);
            return;
        }
    if (om && om->map_d[0] && om->dHandle >= 0) {
        AlphaParams a = {om->dHandle};
        rtCutout(std::strncmp(material, "cobwebs", 7) == 0 ? COBWEB : ALPHA_NEAR, &a, sizeof a);
    } else {
        rtCutout(NEAR_CLIP, NULL, 0);
    }
    /* A Mix Shader whose factor the exporter could not evaluate came out as
     * Ks 0.5, half the light to a glossy lobe (37 materials, the sofa among
     * them, white under kajiya): read them as the 0.12 Fresnel coat the
     * exporter writes when it can read the mix, metals and glass excepted. */
    if (om && std::fabs(om->Ks[0] - 0.5f) < 1e-3f && om->Pm <= 0 && !std::strstr(material, "etal") &&
        !std::strstr(material, "copper") && !std::strstr(material, "lass") && !std::strstr(material, "iquid")) {
        RtMaterial m = ObjMaterialToRt(*om);
        m.specular[0] = m.specular[1] = m.specular[2] = 0.12f;
        rtMaterial(&m);
    }
    /* The counter, the cabinets and the sofa's arms (wood_furniture_generic):
     * the file weights their gloss by a Fresnel term times a scratch map
     * minus 1.73, zero nearly everywhere, which the exporter could not
     * follow and wrote as Ks 0.54, a pale sheen under kajiya. Matte, as the
     * exporter writes the shop's other woods. */
    if (om && std::strncmp(material, "wood_furniture_generic", 22) == 0) {
        RtMaterial m = ObjMaterialToRt(*om);
        m.specular[0] = m.specular[1] = m.specular[2] = 0;
        rtMaterial(&m);
    }
    if (om && std::strcmp(material, "blinds_plastic") == 0) {
        RtMaterial m = ObjMaterialToRt(*om);
        for (int i = 0; i < 3; i++)
            m.emission[i] = BLINDS_RADIANCE * EXPOSURE * blindsScale;
        rtMaterial(&m);
    }
    if (om && std::strcmp(material, "Material") == 0) {
        RtMaterial m = ObjMaterialToRt(*om); /* the file's card: hidden, the cells stand in its place */
        m.emission[0] = m.emission[1] = m.emission[2] = 0;
        rtMaterial(&m);
        rtCutout(HIDDEN, NULL, 0);
    }
    /* The practicals' bulbs and tubes (the file's light_* materials) at
     * the file's emission, which the MTL does not carry: the tube and the
     * chandeliers' bulbs at radiance 2, the sconces' at 2 and warm, the
     * spot lamps' bulbs at 2.5, times the lamp scale (off by default). */
    if (om && std::strncmp(material, "light", 5) == 0 && std::strstr(material, "_off") == NULL) {
        RtMaterial m = ObjMaterialToRt(*om);
        RtColor Le = {2, 2, 2};
        if (std::strncmp(material, "light_wall", 10) == 0) {
            Le[0] = 2.0f; Le[1] = 1.6f; Le[2] = 1.1f;
        } else if (std::strncmp(material, "light_spot", 10) == 0) {
            Le[0] = Le[1] = Le[2] = 2.5f;
        }
        for (int i = 0; i < 3; i++)
            m.emission[i] = Le[i] * EXPOSURE * lampScale;
        rtMaterial(&m);
    }
}

/* lights -------------------------------------------------------------- */

/* Daylight. The seven lamps the file fakes the daylight with are off:
 * they sat just inside the window and blew out the couch. A sun 33
 * degrees up shines in through the window glass (x 1.70 to 3.21 m,
 * y 1.24 to 2.04 m, the front wall at z = -0.78 m) and between the
 * blinds' slats, from a little to the right; the white world is the
 * sky. */
static const Vec3 SUN_DIRECTION(-0.30f, -0.55f, -0.78f); /* the way its light travels */
static const RtColor SUN_IRRADIANCE = {60.0f, 55.0f, 48.0f};
static const Vec3 WINDOW_CENTER(2.45f, 1.64f, -0.78f);

/* Fills for the bounce light a path tracer gives for free and whitted
 * does not: the window's daylight spilling back into the room, each far
 * from what it lights, so none prints a circle. Barbershop only. Their
 * strengths are fitted to the kajiya render of BarbershopSunOnly, not
 * guessed. */
static const struct Fill {
    const char *name;
    RtPoint position;
    RtVector direction;
    RtColor I;
    float cone, penumbra, reach;
} FILLS[] = {
    {"the window's light, back into the room", {2.4f, 2.2f, -1.2f}, {0.05f, -0.35f, -1.0f}, {110.0f, 78.0f, 50.0f}, 70.0f, 40.0f, 0},
    {"the window wall, from the back of the room", {2.0f, 1.8f, -8.5f}, {0.05f, -0.1f, 1.0f}, {4.6f, 3.7f, 3.2f}, 50.0f, 30.0f, 0},
};

/* The ceiling's bounce on the floor and the furniture, which lights the
 * room from above under kajiya: a grid of hidden down-spots just under
 * the ceiling, two rows down the room, each with a wide cone, far from
 * what it lights and overlapping its neighbors, so none prints a circle.
 * Barbershop only; fitted, as the fills above. */
static const RtColor FLOOR_FILL_I = {3.2f, 2.9f, 2.3f};
static const float FLOOR_FILL_XS[] = {1.2f, 2.8f};
static const float FLOOR_FILL_ZS[] = {-2.0f, -4.0f, -6.0f, -8.0f};
static const float FLOOR_FILL_Y = 2.35f;

/* The shop's own lamps, off by default (LAMP_SCALE_DEFAULT) and at the
 * file's strengths when on (the energy times the Emission node's
 * strength and its color; the seven lamps the file fakes the daylight
 * with stay off, the sun replaces them). The two area lights are
 * emitting quads hidden from the camera and casting no shadow, as
 * Blender hides lamps (LAMP); a spot's cone is half Blender's spot
 * size, its penumbra the blend times that; P watts give I = P / (4 pi). */
static RtInt LAMP(RtShadeContext *c) { return c->depth > 0 ? 1 : 0; }

static const struct AreaLight {
    const char *name;
    Vec3 center, right, up;
    float width, height;
    RtColor Le;
} AREA_LIGHTS[] = {
    {"counter", {1.876f, 1.926f, -9.990f}, {1, 0, 0}, {0, -0.574f, -0.819f}, 0.5f, 0.1f, {10.77f, 13.63f, 14.99f}},
    {"back", {1.558f, 1.908f, -8.373f}, {0.998f, -0.044f, -0.045f}, {-0.03f, 0.296f, -0.955f}, 0.833f, 0.833f, {0.32f, 0.14f, 0.037f}},
};

static const struct SpotLight {
    const char *name;
    RtPoint position;
    RtVector direction;
    RtColor I;
    float cone, penumbra;
} SPOT_LIGHTS[] = {
    {"chair 2", {3.474f, 2.296f, -7.149f}, {-0.174f, -0.985f, 0}, {3.979f, 2.005f, 1.050f}, 30.0f, 30.0f},
    {"wall front", {0.484f, 1.750f, -5.115f}, {-1, 0, 0}, {0.721f, 0.499f, 0.284f}, 72.95f, 72.95f},
    {"chair 1", {3.474f, 2.296f, -4.554f}, {-0.174f, -0.985f, 0}, {11.94f, 6.016f, 3.151f}, 30.0f, 30.0f},
};

static const struct PointLight {
    const char *name;
    RtPoint position;
    RtColor I;
} POINT_LIGHTS[] = {
    {"chandelier 2", {3.462f, 1.972f, -7.155f}, {6.5f, 4.5f, 1.9f}},
    {"chandelier 1", {3.462f, 1.972f, -4.548f}, {6.5f, 4.5f, 1.9f}},
};

static void BarbershopLights(bool fills)
{
    const RtColor E = {SUN_IRRADIANCE[0] * EXPOSURE * sunScale, SUN_IRRADIANCE[1] * EXPOSURE * sunScale,
                       SUN_IRRADIANCE[2] * EXPOSURE * sunScale};
    SunDisc(normalize(SUN_DIRECTION), E, 0.53f, WINDOW_CENTER); /* the sun's own angular size, 50 m out */
    {
        /* the white world behind the blinds, as a grid of cells (above);
         * each faces ev x eu = -z, into the room */
        const float w = (WINDOW_X1 - WINDOW_X0) / WINDOW_COLS, h = (WINDOW_Y1 - WINDOW_Y0) / WINDOW_ROWS;
        const float k = EXPOSURE * backdropScale;
        RtMaterial m = EMITTER(BACKDROP_RADIANCE[0] * k, BACKDROP_RADIANCE[1] * k, BACKDROP_RADIANCE[2] * k);
        rtMaterial(&m);
        rtCutout(PANE, NULL, 0);
        for (int r = 0; r < WINDOW_ROWS; r++)
            for (int c = 0; c < WINDOW_COLS; c++)
                Quad(Vec3(WINDOW_X0 + c * w, WINDOW_Y0 + r * h, WINDOW_Z), Vec3(w, 0, 0), Vec3(0, h, 0));
        rtCutout(NULL, NULL, 0);
    }
    for (size_t i = 0; i < sizeof AREA_LIGHTS / sizeof AREA_LIGHTS[0]; i++) {
        const AreaLight &a = AREA_LIGHTS[i];
        const float k = EXPOSURE * lampScale;
        RtMaterial m = EMITTER(a.Le[0] * k, a.Le[1] * k, a.Le[2] * k);
        rtMaterial(&m);
        rtCutout(LAMP, NULL, 0);
        const Vec3 eu = a.right * a.width, ev = a.up * a.height;
        Quad(a.center - eu * 0.5f - ev * 0.5f, eu, ev); /* faces up x right: the lamp's -z */
    }
    rtCutout(NULL, NULL, 0);
    for (size_t i = 0; i < sizeof SPOT_LIGHTS / sizeof SPOT_LIGHTS[0]; i++) {
        const SpotLight &s = SPOT_LIGHTS[i];
        RtPoint pos = {s.position[0], s.position[1], s.position[2]};
        RtVector dir = {s.direction[0], s.direction[1], s.direction[2]};
        RtColor I = {s.I[0] * EXPOSURE * lampScale, s.I[1] * EXPOSURE * lampScale, s.I[2] * EXPOSURE * lampScale};
        rtSpotLight(pos, dir, I, s.cone, s.penumbra, 0);
    }
    for (size_t i = 0; i < sizeof POINT_LIGHTS / sizeof POINT_LIGHTS[0]; i++) {
        const PointLight &p = POINT_LIGHTS[i];
        RtPoint pos = {p.position[0], p.position[1], p.position[2]};
        RtColor I = {p.I[0] * EXPOSURE * lampScale, p.I[1] * EXPOSURE * lampScale, p.I[2] * EXPOSURE * lampScale};
        rtPointLight(pos, I);
    }
    for (size_t i = 0; fills && i < sizeof FILLS / sizeof FILLS[0]; i++) {
        const Fill &f = FILLS[i];
        RtPoint pos = {f.position[0], f.position[1], f.position[2]};
        RtVector dir = {f.direction[0], f.direction[1], f.direction[2]};
        RtColor I = {f.I[0] * EXPOSURE, f.I[1] * EXPOSURE, f.I[2] * EXPOSURE};
        rtSpotLight(pos, dir, I, f.cone, f.penumbra, f.reach);
    }
    for (size_t ix = 0; fills && ix < sizeof FLOOR_FILL_XS / sizeof FLOOR_FILL_XS[0]; ix++)
        for (size_t iz = 0; iz < sizeof FLOOR_FILL_ZS / sizeof FLOOR_FILL_ZS[0]; iz++) {
            RtPoint pos = {FLOOR_FILL_XS[ix], FLOOR_FILL_Y, FLOOR_FILL_ZS[iz]};
            RtVector down = {0, -1, 0};
            RtColor I = {FLOOR_FILL_I[0] * EXPOSURE, FLOOR_FILL_I[1] * EXPOSURE, FLOOR_FILL_I[2] * EXPOSURE};
            rtSpotLight(pos, down, I, 75.0f, 35.0f, 0);
        }
}

/* scene -------------------------------------------------------------- */

/* The scene, with the fills and the ambient (Barbershop) or with the sun
 * and the window's light alone (BarbershopSunOnly). */
static void BarbershopScene(const char *name, bool fills)
{
    /* The camera: a 13 mm lens on a 24.9 mm sensor, 87.5 degrees wide,
     * 56.6 degrees tall at 16:9 (the file frames 2.39:1 at the same
     * width); f/1.4 focused 5.8 m out, which whitted ignores. */
    RtPoint eye = {2.968f, 1.545f, -9.955f}, lookat = {1.552f, 1.287f, -5.167f};
    RtVector up = {-0.021f, 0.999f, 0.048f};
    RtColor white = {EXPOSURE, EXPOSURE, EXPOSURE};
    RtColor ka = {0.175f * EXPOSURE, 0.180f * EXPOSURE, 0.100f * EXPOSURE}; /* fitted with the fills */
    RtColor none = {0, 0, 0};

    lampScale = rtSceneArg(0, LAMP_SCALE_DEFAULT);
    sunScale = rtSceneArg(1, 1.0f);
    backdropScale = rtSceneArg(2, 1.0f);
    blindsScale = rtSceneArg(3, 1.0f);
    rtBegin();
        rtFormat(1920, 1080);
        rtDisplay(name);
        rtPixelSamples(2048); /* cook and kajiya: the chrome and the mirrors reflect the x1000 window, which
                                 takes many samples to settle; whitted renders one sample */
        rtSampler("halton");
        rtTonemap("filmic", 1.0f);
        rtCamera(eye, lookat, up, 56.6f);
        rtLens(0.0046f, 5.8f);
        rtMaxDepth(6);
        rtAmbient(fills ? ka : none);
        rtBackground(white);
        rtWorldBegin(0);
            BarbershopLights(fills);
            ObjMesh *model = obj_load("models/barbershop/barbershop.mesh");
            BarbershopHook hook = {model};
            ObjDraw(model, 0, BarbershopMaterials, &hook);
            obj_free(model);
            BarbershopHook none_ = {NULL};
            ObjDrawInstances("models/barbershop/instances.txt", 0, BarbershopMaterials, &none_);
        rtWorldEnd();
    rtEnd();
}

void Barbershop(void) { BarbershopScene("3_HeroScenes/Barbershop", true); }
void BarbershopSunOnly(void) { BarbershopScene("3_HeroScenes/BarbershopSunOnly", false); }
