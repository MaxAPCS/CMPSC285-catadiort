/*
 * scenes.h: the tables of scenes the executable can render, by name. The
 * scene functions are C++; a renderer in any language finds them through
 * these C-linkage tables, one per folder: scenes/1_Whitted/0_scene_table.cpp
 * defines RT_SCENES_HW2 (always there), scenes/2_Cook/ and scenes/3_Kajiya/ define
 * RT_SCENES_HW3 and RT_SCENES_HW4 (from their kits on), scenes/hidden/
 * RT_SCENES_HIDDEN. The build defines RT_HAVE_SCENES_HW3 and so on when
 * the folder's table file exists, and main reads a table only then.
 */
#ifndef SCENES_H
#define SCENES_H

#include <stddef.h> /* NULL */

#ifdef __cplusplus
extern "C" {
#endif

/* A scene whose name ends in "Movie" renders numbered frames, <tier
 * folder>/<name>/<name>_000.png and so on (one rtWorldBegin/End and one
 * rtDisplay per frame), which ffmpeg turns into an MP4; gauchoRT stills skips
 * them. A still twin without the suffix usually renders one frame. */
typedef struct RtSceneEntry {
    const char *name;       /* what you type: e.g., gauchoRT Spheres */
    void (*function)(void); /* the scene's entry point (function pointer) */
    const char *tier;       /* "test", "benchmark", "study", "compare", "hero" or
                               "extra" (the scenes of a homework's extra-credit
                               features): which folder of scenes/<integrator>/ it is
                               in, and of renders/<integrator>/ its image goes to */
} RtSceneEntry;

/* {name, function, tier} entries, each table ending with {NULL, NULL, NULL} */
extern const RtSceneEntry RT_SCENES_HW2[];
extern const RtSceneEntry RT_SCENES_HW3[];
extern const RtSceneEntry RT_SCENES_HW4[];
extern const RtSceneEntry RT_SCENES_HIDDEN[];

#ifdef __cplusplus
}
#endif

#endif /* SCENES_H */
