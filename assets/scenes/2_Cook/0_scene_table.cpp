/*
 * 0_scene_table.cpp: every HW3 scene gauchoRT can render, by name
 * and tier: the test scenes (1_TestScenes/, a file per scene, in the
 * Suggested Workflow's order), then 2_StudyScenes/, then 3_HeroScenes/,
 * then 4_ExtraCreditScenes/, the scenes of the extra-credit features
 * (tier "extra"). Not a scene file: this table is how main.cpp (or
 * main.rs) reaches the scene functions. To add a scene, declare its
 * function here and add one line to RT_SCENES_HW3. The HW2 scenes keep
 * their own folder and table (scenes.h).
 */
#include "scenes.h"

/* 1_TestScenes/ */
void Stripes(void);        /* stripes from 4 pixels wide to half a pixel: the stratified sampler on the pixel's area */
void Stripes_halton(void); /* the same under halton */
void WarpView(void); /* a warp's samples as red spheres on its domain, from the dumps tools/warptest.py writes */
void ScatteringPebbles(void); /* the thin lens at f/2 over forty thousand pebbles */
void SphereLight(void);
void SphereLightInside(void); /* a lamp's sphere around what it lights */
void QuadLight(void);  /* one square lamp over a floor: tools/area_light_test.py's exact image */
void PerforatedLamp(void); /* a lamp with holes cut by a cutout, beside a solid one */
void ShadowRays(void); /* a lamp over a card at one camera ray a pixel: eye rays against shadow rays (tools/light_samples.py) */
void AmbientOcclusion(void);
void GlossyFurnace(void);
void MirrorLamp(void);        /* a smooth and a rough plate under a small lamp and a point light */
void FrostedEarth(void);      /* a glass globe, oceans clear and continents frosted: glossy transmission */
void FrostedEarthMovie(void); /* the globe turning, 216 frames */
void Convergence(void);

/* 2_StudyScenes/ */
void FocusSweep(void); /* eight frames, into 2_StudyScenes/FocusSweep/ */
void ManyLights(void);

/* 3_HeroScenes/ */
void MagicalClock(void);
void ChopperTitan(void);
void Candy(void);

/* 4_ExtraCreditScenes/ (motion blur: MotionBlur.cpp) */
void MotionBlur(void);
void MotionBlurPan(void);      /* the same with the camera keyed */
void Pool1984(void);           /* the scene 1984: a C name cannot start with a digit */
void MoleculeFastMovie(void);  /* HW2's molecule at 10 turns, 96 frames */

extern "C" const RtSceneEntry RT_SCENES_HW3[] = {
    /* 1_TestScenes/ */
    {"Stripes", Stripes, "test"},
    {"Stripes_halton", Stripes_halton, "test"},
    {"WarpView", WarpView, "test"},
    {"ScatteringPebbles", ScatteringPebbles, "test"},
    {"SphereLight", SphereLight, "test"},
    {"SphereLightInside", SphereLightInside, "test"},
    {"QuadLight", QuadLight, "test"},
    {"PerforatedLamp", PerforatedLamp, "test"},
    {"ShadowRays", ShadowRays, "test"},
    {"AmbientOcclusion", AmbientOcclusion, "test"},
    {"GlossyFurnace", GlossyFurnace, "test"},
    {"MirrorLamp", MirrorLamp, "test"},
    {"FrostedEarth", FrostedEarth, "test"},
    {"FrostedEarthMovie", FrostedEarthMovie, "test"},
    {"Convergence", Convergence, "test"},
    /* 2_StudyScenes/ */
    {"FocusSweep", FocusSweep, "study"},
    {"ManyLights", ManyLights, "study"},
    /* 3_HeroScenes/ */
    {"MagicalClock", MagicalClock, "hero"},
    {"ChopperTitan", ChopperTitan, "hero"},
    {"Candy", Candy, "hero"},
    /* 4_ExtraCreditScenes/ */
    {"MotionBlur", MotionBlur, "extra"},
    {"MotionBlurPan", MotionBlurPan, "extra"},
    {"1984", Pool1984, "extra"},
    {"MoleculeFastMovie", MoleculeFastMovie, "extra"},
    {NULL, NULL, NULL}
};
