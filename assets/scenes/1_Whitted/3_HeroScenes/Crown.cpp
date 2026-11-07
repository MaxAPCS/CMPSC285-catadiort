/*
 * ECE/CS 285 – HW2 Hero Scene: Crown
 * The Austrian Imperial Crown on red velvet: gold, pearls, enamel and
 * cut gems (rubies, sapphires, diamonds, zircons, all refracting), lit
 * by six warm panels (emitting quads, point lights under whitted) in a
 * grey studio; 2.7 million triangles and 632 pearls as spheres.
 * 1080 x 1512 (the file's portrait frame), 256 spp, halton, seed 0
 * under cook and kajiya.
 * Model: the Imperial Crown of Austria by Martin Lubich (loramel.net),
 * in the pbrt-v3 scenes' version.
 *
 * Output: renders/<integrator>/3_HeroScenes/Crown.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

#include <cstdio>
#include <cstring>

/* Everything scaled by this. */
static const float EXPOSURE = 1.0f;

/* The colored gems: the file tints their glass, which an rtMaterial
 * cannot, so they keep a little of a body color under their glass. */
static const struct Gem {
    const char *material;
    RtColor body;
} GEMS[] = {
    {"big_ruby", {0.5f, 0.02f, 0.02f}},
    {"small_ruby", {0.5f, 0.02f, 0.02f}},
    {"saphire", {0.08f, 0.18f, 0.6f}},
};

static void CrownMaterials(const char *material, void *user)
{
    const ObjMesh *mesh = (const ObjMesh *)user;
    for (size_t k = 0; k < sizeof GEMS / sizeof GEMS[0]; k++) {
        if (std::strcmp(material, GEMS[k].material) != 0)
            continue;
        for (int i = 0; i < mesh->nmaterials; i++)
            if (std::strcmp(mesh->materials[i].name, material) == 0) {
                RtMaterial m = ObjMaterialToRt(mesh->materials[i]);
                for (int c = 0; c < 3; c++)
                    m.albedo[c] = GEMS[k].body[c];
                m.transmission = 0.7f;
                rtMaterial(&m);
            }
    }
}

/* The pearls: 632 of the file's 1,025, the ball-shaped ones (873,000
 * triangles), are spheres, listed in models/crown/pearls.txt (x y z r a
 * line) and drawn here in the file's pearl material; the rest of its
 * pearls are in the mesh. */
static void Pearls(const ObjMesh *mesh)
{
    for (int i = 0; i < mesh->nmaterials; i++)
        if (std::strcmp(mesh->materials[i].name, "pearl") == 0) {
            RtMaterial m = ObjMaterialToRt(mesh->materials[i]);
            rtMaterial(&m);
        }
    rtSurface(NULL, NULL, 0);
    rtCutout(NULL, NULL, 0);
    FILE *f = std::fopen("models/crown/pearls.txt", "r");
    if (!f) {
        std::fprintf(stderr, "Crown: models/crown/pearls.txt is missing\n");
        return;
    }
    char line[256];
    while (std::fgets(line, sizeof line, f)) {
        float x, y, z, r;
        if (std::sscanf(line, "%f %f %f %f", &x, &y, &z, &r) != 4)
            continue; /* the comment */
        rtTransformBegin();
            rtTranslate(x, y, z);
            rtSphere(r);
        rtTransformEnd();
    }
    std::fclose(f);
}

void Crown(void)
{
    /* The camera: 62.7 degrees tall on the portrait frame, a 0.3 lens
     * focused 17 ahead, on the crown (whitted ignores the lens). */
    RtPoint eye = {0, 5.5f, 24.0f}, lookat = {0, 11.0f, -10.0f};
    RtVector up = {0, 1, 0};
    RtColor ka = {0.03f * EXPOSURE, 0.03f * EXPOSURE, 0.03f * EXPOSURE};
    RtColor black = {0, 0, 0};
    RtMaterial studio = MATTE(0.2f, 0.2f, 0.2f);

    rtBegin();
        rtFormat(1080, 1512);
        rtDisplay("3_HeroScenes/Crown");
        rtTonemap("filmic", 1.0f);
        rtCamera(eye, lookat, up, 62.66f);
        rtLens(0.3f, 17.0f);
        rtPixelSamples(256);
        rtSampler("halton");
        rtMaxDepth(12);
        rtAmbient(ka);
        rtBackground(black);
        rtWorldBegin(0);
            /* The studio: a grey sphere around everything, lit by the
             * panels, which the gold and the pearls reflect. */
            rtMaterial(&studio);
            rtSphere(100);
            ObjMesh *model = obj_load("models/crown/crown.mesh");
            ObjDraw(model, 0, CrownMaterials, model);
            Pearls(model);
            obj_free(model);
        rtWorldEnd();
    rtEnd();
}
