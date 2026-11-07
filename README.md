# ECE/CS 285 HW2: a Whitted ray tracer

This is the starter kit for HW2. It builds and runs as it is: until you
fill in the stubs, every picture is black. Your job is to replace the
stubs in `src/rt.cpp` with a ray tracer, following
the handout and the Suggested Workflow. `reference_renders/` has every
scene as the reference renderer draws it: that is what your images should
look like. The same renderer continues through HW3 and HW4; the
interface in `include/rt.h` is complete now and never changes.

## The three downloads

The kit is three zips that unzip into the same folder: `hw2.zip`, the
core kit (the code, the test and benchmark scenes, their models and
textures, their reference renders, about 100 MB), and `hw2_heroA.zip`
and `hw2_heroB.zip`, the hero scenes' models, textures and reference
renders (350 to 450 MB each). The hero scenes' source files are in the
core kit, so they build without the packs and fail at load time until the
pack they need is unzipped over the kit. Everything else works with the
core kit alone.

## Build and run

The renderer may be written in C++ or Rust. The scene files are C++
either way, and everything they call has C linkage, so both link against
the same unchanged scenes.

**C++.** You need CMake (3.16 or newer) and a C++17 compiler (gcc, clang
or Visual Studio).

    cmake -B build
    cmake --build build
    ./build/gauchoRT Spheres

`CMakeLists.txt` picks a Release (optimized) build when no build type is
given, so these three lines are all there is on Linux and macOS. Visual
Studio builds every configuration from one project, so on Windows name
the configuration when building and running:

    cmake -B build
    cmake --build build --config Release
    .\build\Release\gauchoRT.exe Spheres

CMake takes the first toolchain it finds. When that is not Visual Studio
(MSYS2's gcc with Ninja, say, on a machine that has both), its first
lines say so (`-- Building for: Ninja`), the configuration flag is
ignored, the executable is `.\build\gauchoRT.exe`, and it runs from the
shell whose PATH found that toolchain, where its runtime DLLs are.

Either way, run the executable from this folder, the one that contains
`scenes/`, `textures/`, `models/` and `renders/`: that is where it looks
for models and textures and where it writes the images. The build takes
every `.cpp` under `src/`, so split your renderer into files as it grows;
nothing in `CMakeLists.txt` needs editing.

**Rust.** You need a Rust toolchain (rustup) and a C++17 compiler for the
scene files. `rust/` is a Cargo project: `build.rs` compiles `../scenes`
with the `cc` crate, `src/rt.rs` is the stub (the same types as `rt.h`,
`#[repr(C)]`, and the same functions exported with `#[no_mangle] extern
"C"`), and `src/shaders.rs`, `src/texture.rs`, `src/main.rs`,
`src/cmdline.rs`, `src/options.rs`, `src/transform.rs`, `src/image.rs`,
`src/noise.rs` and `src/debug.rs` are the given files, the twins of the
C++ ones.

    cd rust
    cargo run --release -- Spheres

The executable is `rust/target/release/gauchoRT`; it changes to the kit
root by itself, so it can be run from anywhere.

## The command line

    gauchoRT <scene> [numbers]   one scene by name (gauchoRT alone lists them); the numbers
                                 after the name are the scene's own arguments (rtSceneArg)
    gauchoRT all                 every scene
    gauchoRT stills              every scene but the movies
    gauchoRT test | benchmark | hero
                                 one tier
    gauchoRT --help              every option

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

## What is where

    include/rt.h          the interface your renderer implements; do not change it
    include/shaders.h     the four shaders and their parameter structs; do not change it
    include/scenes.h      the table of scenes (filled in by scenes/*/0_scene_table.cpp)
    include/debug.h       the trace log and the debug images' encoders, given complete
    include/timer.h       the stopwatch and the time budget of --time-limit, given complete
    include/*.h (others)  helpers the scene files use: obj.h loads the models,
                          sceneutil.h is shared scene code, vec.h a small vector type
                          (yours to use too, or any vector library you like)
    src/main.cpp          main(): runs the scenes the command line names; given (the one
                          edit is its SIMPLE_MAIN switch, Suggested Workflow steps 3 and 8)
    src/cmdline.*         the command line (every option) and the scene tables; given
    src/options.h         the options struct main fills and your renderer reads, and
                          RT_HOMEWORK, the one line that changes when HW3 starts
    src/transform.*       the current transformation (rtTranslate and friends); given
    src/image.*           writeImage: the PNG with its tone map, the PFM; readImage: the
                          textures' files (PNG, JPEG, HDR, PFM) as linear floats; given
    src/noise.cpp         perlinNoise; given
    src/debug.cpp         --trace; given
    src/rt.cpp            the renderer: stubs for you to fill in
    src/shaders.cpp       TEXTURED, CHECKER, ALPHA_CUTOUT, CHECKER_CUTOUT; given (HW1 had them)
    src/texture.*         the texture table and its lookup behind rtMakeTexture and rtTexture; given
    rust/                 the same kit for Rust
    scenes/1_Whitted/     the scene files, in three tiers (see below)
    renders/              where your images go, in the same folders
    reference_renders/    the same images from the reference renderer:
                          compare yours against these (tools/compare.py)
    textures/, models/    what the scenes load
    third_party/          stb_image.h and stb_image_write.h
    tools/compare.py      RMSE and a difference image between two PNGs or two folders
    tools/timing_table.py the write-up's timing table over the benchmark and hero scenes

## The scenes

`scenes/1_Whitted/1_TestScenes/` holds the small test scenes, one per
file, in the order the Suggested Workflow needs them; each one's comment
says what it shows and tests, and `reference_renders/` shows the picture.
Start with `Spheres`. `2_BenchmarkScenes/` holds the three scenes the
leaderboard times (`Dragon`, `Stadium`, `Forest`), and `3_HeroScenes/`
the big ones: `Sponza`, `WhiteLands`, `BarcelonaPavilion`, `SanMiguel`,
`Crown`, `Barbershop`, `Chess`, `FishyCat` and `AtticCloseup`, plus
`BarcelonaPavilionSunOnly`, lit by its sun alone, and
`BarbershopSunOnly`, lit by the sun and the daylight through its window
alone, nothing hidden. The scene files are the inputs to your
renderer and must not be edited (the handout says what to hand in). To
add a scene of your own, put it in a new file under `scenes/1_Whitted/`
(the build finds it) and add one line to `scenes/1_Whitted/0_scene_table.cpp`.

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

`build` is the acceleration structure, `render` the pixel loop, `total`
includes writing the image. The leaderboard runs the benchmark and hero
scenes at 1920 x 1080 on 8 threads, three times each, and ranks the
median of `build + render` (not `total`, which includes the given PNG
writer) by its geometric mean over the scenes, after checking each image
against the reference with `tools/compare.py`. The handout has the
details and the threshold. The reference renderer's own times are on the
leaderboard too; it instances a mesh drawn many times (Forest's trees)
instead of baking every copy, which is extra credit for you, and so is
beating its times. `--no-accel` renders without the acceleration
structure, for the timing table of the write-up; `--time-limit S` makes
that affordable: brute force would take hours to days per frame, so it
renders for S seconds and its `LIMIT` line estimates the whole frame.
`python tools/timing_table.py` produces that table: every benchmark and
hero scene at the benchmark's settings with the SAH builder, the
midpoint builder, and brute force for five minutes, whose estimate is
the brute-force column (`--runs 3` for the benchmark's median of the
builders); the Suggested Workflow's step 18 says when.

## Comparing against the reference

    python tools/compare.py renders/1_Whitted reference_renders/1_Whitted

compares every image you have rendered against its reference: the RMSE
of the pixel values, pass or fail against the scene's threshold, and a
difference image beside each failing one. `--aov normal` and the other
debug images, and `--trace X Y` with the reference library when it is
made available, are the tools for finding out why.

## AI tools

The handout's AI policy is HW1's: an AI tool may explain concepts, help
you debug, interpret errors, and write setup, build files, image I/O and
scripts. The renderer is yours to implement: the camera, the
intersections, the BVH, the lights and shadows, the Whitted model, the
shaders and textures. The handout says what to hand in and how to
acknowledge what a tool did.
