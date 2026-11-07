/*
 * ECE/CS 285 – HW2 Hero Scene: SanMiguel
 * The courtyard of the San Miguel hacienda, from the arcade: tables laid
 * for lunch under the tree, ivy on the walls, potted plants, the gallery
 * above, under a noon sun with the sky as the backdrop. 10.3 million
 * triangles (the leaves and tableware through instances.txt), 295
 * materials, 97 of them alpha cutouts. 1440 x 1080, filmic at +3.5;
 * 256 spp, halton, seed 0 under cook and kajiya.
 * Model: San Miguel by Guillermo M. Leal Llaguno (Evolución Visual),
 * from pbrt-v4-scenes (pbrt.org), CC-BY; its sky in
 * textures/env/sanmiguel_sky.hdr.
 *
 * Output: renders/<integrator>/3_HeroScenes/SanMiguel.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

void SanMiguel(void)
{
    /* pbrt's "courtyard" camera (LookAt after Scale -1 1 1, so the view
     * is ours as given), fov 84 degrees on the shorter axis of its
     * cropped 1120 x 986 frame; here 4:3 at the same height. */
    RtPoint eye = {22.8676f, 1.94784f, 12.9289f}, lookat = {22.2116f, 2.04755f, 12.1807f};
    RtVector up = {0.065738f, 0.995016f, 0.0749807f};

    /* The sun, measured on the sky map (pbrt's infinite light at scale
     * 15): irradiance (18.4, 14.4, 10.2) within a disc 0.94 degrees
     * across, 70 degrees up from the north-east. The sky without it
     * (mean radiance (0.97, 1.25, 1.79)) is the backdrop and, as whitted
     * has no sky light, the ambient. */
    const float EXPOSURE = 0.3f;
    RtVector sunDir = {0.2808f, -0.9394f, -0.1966f};
    RtColor sunE = {18.43f * EXPOSURE, 14.45f * EXPOSURE, 10.18f * EXPOSURE};
    RtColor ka = {0.30f * EXPOSURE, 0.27f * EXPOSURE, 0.24f * EXPOSURE}; /* under a tenth of the sky's, warm: the courtyard shade stays shade */

    rtBegin();
        rtFormat(1440, 1080);
        rtDisplay("3_HeroScenes/SanMiguel");
        rtTonemap("filmic", 3.5f); /* the shade under the tree and arcades opens, the sun rolls off */
        rtCamera(eye, lookat, up, 83.97f);
        rtMaxDepth(6);
        rtPixelSamples(256); /* cook and kajiya; whitted renders one sample */
        rtSampler("halton");
        rtAmbient(ka);
        rtEnvironment("env/sanmiguel_sky.hdr", EXPOSURE, 0);
        rtWorldBegin(0);
            /* The opening the sky comes in by: the building's footprint at
             * the roofs' ridge (11.1 m; the courtyard inside it is open from
             * x 8 to 23 and z 1 to 13), so every sky ray that clears a roof
             * edge crosses it: the opening a sky sampler may draw its
             * directions through (extra credit from HW3). No integrator
             * reads it yet. */
            const RtPoint portal[4] = {{6, 11.1f, -4}, {27, 11.1f, -4}, {27, 11.1f, 13.5f}, {6, 11.1f, 13.5f}};
            rtEnvironmentPortal(portal);
            SunDisc(Vec3(sunDir), sunE, 0.94f, Vec3(22.2f, 2.0f, 12.2f), 200.0f); /* the map's sun, 0.94 degrees, 200 m out */
            ObjMesh *model = obj_load("models/sanmiguel/sanmiguel.mesh");
            ObjDraw(model);
            obj_free(model);
            ObjDrawInstances("models/sanmiguel/instances.txt");
        rtWorldEnd();
    rtEnd();
}
