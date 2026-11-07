/*
 * 0_scene_table.cpp: every HW2 scene gauchoRT can render, by name
 * and tier: 1_TestScenes/ (a file per scene, related scenes sharing one)
 * in the order the Suggested Workflow needs them, then 2_BenchmarkScenes/
 * and 3_HeroScenes/. Not a scene file: main.cpp (or the Rust main.rs)
 * reaches the scene functions through this table. To add a scene,
 * declare its function here and add one line to RT_SCENES_HW2. HW3 and
 * HW4 have their own folders and tables (scenes.h).
 */
#include "scenes.h"

/* 1_TestScenes/ */
void Spheres(void);
void Ellipsoids(void);
void WhittedSpheres(void);
void CornellBox(void);
void CornellBoxSpheres(void);
void Earth(void);
void Cutout(void);
void Nested(void);
void Bunny(void);
void BallOnSlab(void);
void Molecule(void);
void MoleculeMovie(void); /* 96 frames, into 1_TestScenes/MoleculeMovie/ */
void Blocks(void);
void FunWithRefraction(void);
void FunWithRefractionMovie(void); /* 96 frames, into 1_TestScenes/FunWithRefractionMovie/ */
void VertexColors(void);
void Lucy(void);
void Skylight(void);
void Paperclip(void);
void Aliasing(void);
void GlossySweep(void); /* the roughness sweep: under whitted the mirror ray's fade, under cook the lobe */
void Aliasing_64(void); /* the same scene at 64 spp: a cook image (whitted's would be Aliasing's) */
void DepthOfField_near(void);
void DepthOfField_mid(void);
void DepthOfField_far(void);

/* 2_BenchmarkScenes/ */
void Dragon(void);
void Stadium(void);
void Forest(void);

/* 3_HeroScenes/ */
void Sponza(void);
void WhiteLands(void);
void BarcelonaPavilion(void);
void BarcelonaPavilionSunOnly(void); /* BarcelonaPavilion lit by the sun and sky alone, for kajiya */
void SanMiguel(void);
void Crown(void);
void Barbershop(void);
void BarbershopSunOnly(void);    /* the sun and the window's light alone, nothing hidden */
void Chess(void);
void FishyCat(void);
void AtticCloseup(void);

extern "C" const RtSceneEntry RT_SCENES_HW2[] = {
    /* 1_TestScenes/ */
    {"Spheres", Spheres, "test"},
    {"Ellipsoids", Ellipsoids, "test"},
    {"WhittedSpheres", WhittedSpheres, "test"},
    {"CornellBox", CornellBox, "test"},
    {"CornellBoxSpheres", CornellBoxSpheres, "test"},
    {"Earth", Earth, "test"},
    {"Cutout", Cutout, "test"},
    {"Nested", Nested, "test"},
    {"Bunny", Bunny, "test"},
    {"BallOnSlab", BallOnSlab, "test"},
    {"Molecule", Molecule, "test"},
    {"MoleculeMovie", MoleculeMovie, "test"},
    {"Blocks", Blocks, "test"},
    {"FunWithRefraction", FunWithRefraction, "test"},
    {"FunWithRefractionMovie", FunWithRefractionMovie, "test"},
    {"VertexColors", VertexColors, "test"},
    {"Lucy", Lucy, "test"},
    {"Skylight", Skylight, "test"},
    {"Paperclip", Paperclip, "test"},
    {"Aliasing", Aliasing, "test"},
    {"GlossySweep", GlossySweep, "test"},
    {"Aliasing_64", Aliasing_64, "test"},
    {"DepthOfField_near", DepthOfField_near, "test"},
    {"DepthOfField_mid", DepthOfField_mid, "test"},
    {"DepthOfField_far", DepthOfField_far, "test"},
    /* 2_BenchmarkScenes/ */
    {"Dragon", Dragon, "benchmark"},
    {"Stadium", Stadium, "benchmark"},
    {"Forest", Forest, "benchmark"},
    /* 3_HeroScenes/ */
    {"Sponza", Sponza, "hero"},
    {"WhiteLands", WhiteLands, "hero"},
    {"BarcelonaPavilion", BarcelonaPavilion, "hero"},
    {"BarcelonaPavilionSunOnly", BarcelonaPavilionSunOnly, "hero"},
    {"SanMiguel", SanMiguel, "hero"},
    {"Crown", Crown, "hero"},
    {"Barbershop", Barbershop, "hero"},
    {"BarbershopSunOnly", BarbershopSunOnly, "hero"},
    {"Chess", Chess, "hero"},
    {"FishyCat", FishyCat, "hero"},
    {"AtticCloseup", AtticCloseup, "hero"},
    {NULL, NULL, NULL}
};
