/*
 * ECE/CS 285 – HW3 Hero Scene: ChopperTitan
 * A steampunk chopper on a dark polished floor under a sunset sky: chrome
 * and brushed metal everywhere, glossy paint, a low red sun 8 degrees
 * across (an emitting disc) and the sky as a backdrop in every
 * reflection; 4.5 million triangles. 1920 x 1080, 256 spp, halton, seed 0.
 * Model: Chopper Titan by julioras3d (blendswap.com, CC-BY 3.0), in the
 * pbrt-v3 scenes' version; its sky in textures/env/choppertitan_sky.hdr.
 *
 * Output: renders/<integrator>/3_HeroScenes/ChopperTitan.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

#include <cstring>

/* Everything scaled by this: the file clamps linear radiance at 20. */
static const float EXPOSURE = 1.0f;

/* The floor: the file's dark substrate with a 0.1 gloss, which the
 * model's MTL does not carry. */
static void ChopperMaterials(const char *material, void *user)
{
    const ObjMesh *mesh = (const ObjMesh *)user;
    if (std::strcmp(material, "coateddiffuse") != 0) {
        return;
    }
    for (int i = 0; i < mesh->nmaterials; i++) {
        if (std::strcmp(mesh->materials[i].name, material) == 0) {
            RtMaterial m = ObjMaterialToRt(mesh->materials[i]);
            m.specular[0] = m.specular[1] = m.specular[2] = 0.1f;
            rtMaterial(&m);
        }
    }
}

void ChopperTitan(void)
{
    /* The camera: a 60 degree lens at 3:2, 51.9 degrees tall at 16:9. */
    RtPoint eye = {-3.0f, 1.5f, -10.0f}, lookat = {2.0f, 1.0f, 20.0f};
    RtVector up = {0, 1, 0};

    /* The sun: the sky's brightest spot, 2 degrees up, the sky's own
     * irradiance over its 8 degrees (the sky is scaled by 20, as the file
     * has it); the ambient is the rest of the sky on a level surface. */
    const Vec3 chopper(-2.1f, 1.9f, -5.75f);
    RtColor sunE = {1.6f * EXPOSURE, 0.4f * EXPOSURE, 0.02f * EXPOSURE};
    RtColor ka = {0.30f * EXPOSURE, 0.41f * EXPOSURE, 0.43f * EXPOSURE};

    rtBegin();
        rtFormat(1920, 1080);
        rtDisplay("3_HeroScenes/ChopperTitan");
        rtTonemap("filmic", 0.5f);
        rtCamera(eye, lookat, up, 51.9f);
        rtPixelSamples(256);
        rtSampler("halton");
        rtMaxDepth(6);
        rtAmbient(ka);
        rtAmbientOcclusion(0.5f);
        rtEnvironment("env/choppertitan_sky.hdr", 20.0f * EXPOSURE, 80.0f);
        rtWorldBegin(0);
            SunDisc(Vec3(-0.17f, -0.04f, 0.98f), sunE, 8.0f, chopper);
            ObjMesh *model = obj_load("models/choppertitan/choppertitan.mesh");
            ObjDraw(model, 0, ChopperMaterials, model);
            obj_free(model);
        rtWorldEnd();
    rtEnd();
}
