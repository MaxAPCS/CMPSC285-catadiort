/*
 * ECE/CS 285 – HW2 Hero Scene: Sponza
 * The Crytek Sponza atrium (262,267 triangles, about 25 materials with
 * diffuse maps, alpha masks on the plants and chains), seen down the
 * atrium under a sun through the open roof, two point lights and ambient
 * 0.15: TEXTURED and ALPHA_CUTOUT through ObjDraw, long thin shadows, a
 * few mirror rays from the faintly glossy materials; 256 spp, halton, seed
 * 0 under cook and kajiya.
 * Model: Frank Meinl's Sponza for Crytek (2010), as cleaned up by Morgan
 * McGuire (2011), CC BY 3.0; models/sponza/copyright.txt has the details.
 *
 * Output: renders/<integrator>/3_HeroScenes/Sponza.png
 */

#include "rt.h"
#include "obj.h"
#include "sceneutil.h"

void Sponza(void)
{
    RtPoint eye = {-11.5f, 2.3f, -0.8f}, lookat = {4.0f, 2.6f, 0.3f};
    RtVector up = {0, 1, 0};
    RtVector sunDir = {0.35f, -1, 0.3f};
    RtColor sunE = {4, 4, 4}; /* W/m^2 */
    RtPoint lampA = {-6, 5, 0}, lampB = {8, 6, -1};
    RtColor lampI = {120, 120, 120}; /* W/sr */
    RtColor ka = {0.15f, 0.15f, 0.15f};

    rtBegin();
        rtFormat(1920, 1080);
        rtDisplay("3_HeroScenes/Sponza");
        rtPixelSamples(256); /* cook and kajiya; whitted renders one sample */
        rtSampler("halton");
        rtCamera(eye, lookat, up, 55);
        rtAmbient(ka);
        rtBackground(Color8(135, 206, 235));
        rtWorldBegin(0);
            rtDirectionalLight(sunDir, sunE);
            rtPointLight(lampA, lampI);
            rtPointLight(lampB, lampI);

            /* ObjDraw binds each group's material, diffuse map and alpha
             * mask from the MTL. The model is in centimeters. */
            ObjMesh *sponza = obj_load("models/sponza/sponza.mesh");
            rtTransformBegin();
                rtScale(0.01f, 0.01f, 0.01f);
                ObjDraw(sponza);
            rtTransformEnd();
            obj_free(sponza);
        rtWorldEnd();
    rtEnd();
}
