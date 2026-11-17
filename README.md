# CMPSC285 Homework 2 - Whitted Raytracer

## Build
`cargo build --release`

Switches are gated behind compile flags. For brute-force, use `--no-default-features`. For each aov view, use the respective `-F aov___`.

Artifact can be found at [target/release/catadiort](target/release/catadiort)

## Execution
Must be run with assets/ in working directory.

    catadiort <scene> [numbers]   one scene by name (gauchoRT alone lists them); the numbers
                                  after the name are the scene's own arguments (rtSceneArg)
    catadiort all                 every scene
    catadiort stills              every scene but the movies
    catadiort test | benchmark | hero
                                  one tier
    catadiort --help              every option

The options override what a scene asks for, so nothing in a scene file
ever needs editing: `--width W --height H`, `--threads N`, `--no-accel`
(brute force, for the timing table), `--bvh sah|midpoint` (which builder,
once you have two), `--time-limit S` (stop after S seconds and estimate
the whole frame), `--progress`, `--frames A-B` (a
movie's frames A to B only), `--out PATH`, `--pfm` (the linear image
beside the PNG), `--format jpg` (a JPEG instead of the PNG, for sharing
a picture), `--tonemap` and `--exposure`, `--ambient R G B`,
`--max-depth N`, `--aov NAME` (a debug image instead of the picture:
`raydir`, `normal`, `ng`, `depth`, `uv`, `albedo`, `cost`, `shadow`) and `--trace X Y`
(a log of everything the renderer does for one pixel). The options of
HW3 and HW4 (`--spp`, `--sampler`, `--seed`, `--lens`, `--light-samples`,
`--warptest`, `--bsdftest`, `--env-sampling`, `--ISstrategy`) are parsed
now and ignored by a HW2 renderer.

The two scenes that render a movie (`MoleculeMovie` and
`FunWithRefractionMovie`) write numbered frames; `--frames 0-3` renders a
few of them while you work. Turn a finished one into an MP4 with ffmpeg
(the command is in the comment at the top of each scene):

    ffmpeg -framerate 24 -i renders/1_Whitted/1_TestScenes/MoleculeMovie/MoleculeMovie_%03d.png \
           -c:v libx264 -pix_fmt yuv420p -crf 18 renders/1_Whitted/1_TestScenes/MoleculeMovie.mp4

Every scene also carries the settings of HW3 and HW4 (`rtPixelSamples`,
`rtSampler`, `rtLens`, `rtMotionBegin`, `rtAmbientOcclusion`,
`rtEnvironmentPortal`): a HW2 renderer records them and ignores them, as
`rt.h` says, and renders one ray through each pixel's center. They are
there so the same scene renders under the later integrators unchanged.

## The TIME line and the benchmark

`rtWorldEnd` prints one line per image:

    TIME Sponza build=0.412 render=3.871 total=4.283 threads=8

`build` is the acceleration structure, `render` the pixel loop, `total` includes writing the image.

## Comparing against the reference

    python tools/compare.py renders/1_Whitted reference_renders/1_Whitted

compares every image you have rendered against its reference: the RMSE
of the pixel values, pass or fail against the scene's threshold, and a
difference image beside each failing one.

## Footnotes

This project in total was extremely challenging. I may have overscoped by attempting to complete the entire thing with SIMD, and spent a long time perfecting the architecture to ensure everything went smoothly. In particular, it was challenging to hunt down very specific bugs in NaN handling (which I used as a sentinel for empty lanes). I think that I had a good shot, but didn't end up having enough time to complete what I wanted to.

**Extra Credit:**
- Full SIMD Intersections (10)
- Mesh Instancing and Two-Layer BVH (10)
- Tinted Transparent Shadows (3)

AI was used strictly to debug/check for errors. All code written by hand.
