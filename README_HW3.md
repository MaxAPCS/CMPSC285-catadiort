# ECE/CS 285 HW3: a distributed ray tracer

The kit:

- adds the reference renders under `cook`, in `reference_renders/2_Cook/`:
  every HW3 scene, the HW2 scenes `cook` changes, and the grid
  `tools/light_samples.py` makes (`reference_renders/1_Whitted/` stays
  where it is);
- adds two files given complete, `hashing.h` and `ggx_albedo.h`, with
  their Rust twins;
- adds the four study tools and `tools/read_pixel.py` for reading linear PFM pixels;
- brings new versions of four given files, `rt.h`, `image.h`,
  `image.cpp`, and `image.rs`, for one new call (below);
- brings a new time budget, `timer.h` and `timer.rs`, whose time limit
  now stops inside a pixel (below);

## The two edits to your files

The kit adds `src/sampler.cpp` and `src/warp.cpp` (Rust: `sampler.rs`
and `warp.rs`), stubs for you to fill. If your HW2 renderer already has
files of those names, rename yours before you unzip the kit.

Until your sampler exists, `gauchoRT Spheres --integrator whitted`
renders as it did in HW2, and nothing you had stops working.

## The time limit stops inside a pixel

HW2's `--time-limit` let every thread finish the row it was on (with the
fix announced during HW2, the pixel), and under brute force a pixel of a
hero scene takes minutes. The kit's `include/timer.h` (Rust:
`rust/src/timer.rs`) replaces HW2's. When the time is up, a flag goes
up, the searches stop finding hits, and every thread drops the sample it
is in, so a run ends within one ray of its limit and the `LIMIT` line's
estimate counts samples. Under a limit the tiles are single pixels, so
what gets done is scattered over the frame.

Make five edits, the first three in your pixel loop:

1. Take the tile's side from the budget: `budget.tileSize(16)` instead
   of 16 (Rust: `budget.tile_size(16)`). It returns 16 without a limit
   and 1 under one.
2. After each sample, and after `whitted`'s one ray, check
   `budget.expired()`. If it is true, call `budget.cut(s)`, with `s` the
   samples of this pixel finished before this one (0 under `whitted`),
   and stop the worker.
3. Call `budget.done(1)` after each pixel, in place of HW2's call after
   each row (or the patch's after each pixel).
4. At the top of your nearest-hit and any-hit searches, the two
   functions every ray goes through, BVH and brute force alike, add
   `if (RenderBudget::timeUp()) { return false; }` (Rust:
   `if RenderBudget::time_up() { return false; }`). Here `false` stands
   for whatever your search returns for "no hit": a miss record if it
   returns one.
5. Call `budget.report()` after the `TIME` line, with the image's samples
   per pixel as its last argument (`spp`, 1 under `whitted`), where HW2's
   code passed 1.

Do not write the pixel a worker was cut in: leave it the black the image
started with, like every pixel never reached. An image under a limit is
then a scatter of finished pixels; read the `LIMIT` line for the
estimate.

Without `--time-limit` nothing changes, your images included. Your HW2
loop still compiles against the new file and still works without the
edits. With them, this run ends about 30 seconds after the scene has
loaded:

    gauchoRT BarcelonaPavilion --no-accel --time-limit 30 --out renders/try/limit

Then build from scratch. The kit's files keep the zip's date, which is
older than your HW2 build, so an incremental build would not recompile
the new `image.cpp` (or `image.rs`) and would fail to link, or quietly
keep the old one. Delete `build/`, then run:

    cmake -B build
    cmake --build build --config Release

In Rust:

    cargo clean
    cargo build --release

## Three HW2 scenes to replace

The kit's root holds `Paperclip_FIXED.cpp`,
`FunWithRefraction_FIXED.cpp`, and `FishyCat_FIXED.cpp`, corrected copies
of HW2's `scenes/1_Whitted/1_TestScenes/Paperclip.cpp`,
`FunWithRefraction.cpp`, and `scenes/1_Whitted/3_HeroScenes/FishyCat.cpp`
(the last only if you have HW2's hero pack B, which holds FishyCat).

- Copy each over its file, keeping the old name, and delete nothing
  else.
- Never keep both versions of a scene: each defines the same scene, and
  the build would fail.

A comment at the top of each file says what changed. In short:

The Paperclip's old file gave the table's `specular` as 1, meaning
"reflect the clip at full strength" when `specular` only weighted the
mirror ray. Under `cook`, `specular` is the coat's reflectance head on,
and the diffuse under it gets what the coat lets through (the "Cook
Shading Model" handout), so a coat of 1 leaves nothing for the wood and
the table renders black. The corrected file gives the table a specular
of 0.7 and raises the wood's colors to make up for the share the coat
keeps, so that under `cook` the table is a bright oak with the clip's
reflection on it; it also polishes the clip (roughness 0.25 instead of
0.5). Its `whitted` image changes too: the wood is paler, since
`whitted` has no coat and shows the raised colors whole, and the clip
now mirrors the table. The kit brings the new
`reference_renders/1_Whitted/1_TestScenes/Paperclip.png` with it.

FunWithRefraction's old file gave the glass ball and the slab a
roughness of 0.14, which `cook` and `kajiya` render as frosted glass,
with the lights' caustics speckled over everything, where `whitted`'s
Fresnel pair ignores it. The corrected file has clear glass (roughness
0). No reference image changes: `whitted`'s never depended on the
roughness, so HW2's reference stands. What you will see under `kajiya`
in HW4 is the scene's point, not a defect of this file: the pink rod
goes dark inside the slab, where `whitted` draws it, because a path
tracer cannot find a point light or a sun through clear glass.

FishyCat's old file hid the glowing bowl around the cat, its biggest
light, with a cutout that answers "no surface" to every ray. That worked
in HW2 only because the stand-in point light never reads a cutout. In
HW3 a sample of a light asks the light's cutout whether the sampled
point is there (a lamp may have holes: `PerforatedLamp`), so the bowl
would light nothing. The corrected file hides it from the camera and
from shadow rays only (`depth > 0`), as every hidden lamp of the kits
does, so the light sampler and reflected rays see it. Its `whitted`
image changes in the eyes, which now reflect the bowl; the kit brings
the new `reference_renders/1_Whitted/3_HeroScenes/FishyCat.png`.

## What the kit adds

    Paperclip_FIXED.cpp        the corrected HW2 Paperclip (above): copy it over
                               scenes/1_Whitted/1_TestScenes/Paperclip.cpp
    reference_renders/1_Whitted/1_TestScenes/Paperclip.png  its whitted image
    FunWithRefraction_FIXED.cpp  the corrected HW2 FunWithRefraction (above): copy it over
                               scenes/1_Whitted/1_TestScenes/FunWithRefraction.cpp; its whitted
                               reference does not change
    FishyCat_FIXED.cpp         the corrected HW2 FishyCat (above): copy it over
                               scenes/1_Whitted/3_HeroScenes/FishyCat.cpp
    reference_renders/1_Whitted/3_HeroScenes/FishyCat.png  its whitted image
    scenes/2_Cook/             the HW3 scenes, in four tiers (below)
    models/, textures/         what the new scenes load, beside HW2's
    reference_renders/2_Cook/  every HW3 scene under cook, and the HW2 scenes cook changes
    src/hashing.h              the hash and random-number primitives the samplers are built
    rust/src/hashing.rs        on, given complete: include it from your sampler; do not change
                               it (the Rust twin has the same functions in snake_case)
    src/ggx_albedo.h           the glossy lobe's directional albedo E_s(roughness, cosine, F0)
    rust/src/ggx_albedo.rs     and its average, tabulated, given complete: the coat's factor on
                               the diffuse (the Cook Shading Model handout) reads it; do not
                               change it
    include/rt.h               HW2's header plus rtColorGrade (above)
    src/image.h, src/image.cpp the image writer that applies it; given complete, as in HW2
    rust/src/image.rs          (the Rust twin)
    include/timer.h            the time budget whose limit stops inside a pixel (above);
    rust/src/timer.rs          given complete, as in HW2
    src/cmdline.cpp, .h        the command line, given complete, as in HW2: all and the tiers
    rust/src/cmdline.rs        now take only the scenes the integrator has reference images of
    src/sampler.h              the samplers' interface, given complete (the Samplers handout)
    src/sampler.cpp            a stub: write your samplers and makeSampler here
    rust/src/sampler.rs        (Rust: the interface at the top, your samplers below it)
    src/warp.h                 the six warps' names and signatures, given complete
    src/warp.cpp               a stub: write your warps here (the Warps handout)
    rust/src/warp.rs           (the Rust twin of the stub)
    src/warptest.cpp           --warptest, given complete: it prints your warps' samples for
    rust/src/warptest.rs       tools/warptest.py; your warpTest() calls it (workflow step 6)
    tools/warptest.py          the chi-square test of your warps (and --scatter, a look at a
                               sampler's strata through a warp)
    tools/area_light_test.py   the exact test of your area lights, on QuadLight
    tools/light_samples.py     eye rays against shadow rays, on ShadowRays
    tools/convergence.py       the convergence study, on Convergence
    tools/compare.py           a new version, with HW3's thresholds (below)
    README_HW3.md              this file

Compare a folder of your renders with the references:

    python tools/compare.py renders/2_Cook reference_renders/2_Cook

In HW3 it mostly reports an RMSE of zero, or within about a thousandth
of it. The given `hashing.h` is why your `stratified` and `halton`
images can match the reference's to within floating-point error (your
HW2 renderer's own rounding, which the Suggested Workflow's opening
describes), and the workflow names, at every step, the scenes that must.
The kit's `compare.py` has HW3's thresholds: an image under `2_Cook`
passes at an RMSE of 0.002 (0.01 for `Aliasing` and the hero scenes), so
a PASS means a match; HW2's images keep their 0.02.

As in HW2, the reference renderer is also available as a library (the
reference kit, a separate download). Use it to render a scene of your
own, to compare `--trace X Y` logs, and to time your renderer against it
on your own machine.

## What to implement, in order

Follow the Suggested Workflow's seventeen steps. Each gives a goal, what to
do, why, and the check against the reference to pass before you move on:

1. Set `RT_HOMEWORK` to 3, and add a `cook` integrator that starts as a
   copy of your Whitted shading, behind the `--integrator` switch.
2. In the kit's `sampler.cpp`, write `makeSampler` and the `independent`
   sampler (the interface is given, in `sampler.h`).
3. Write the sampled pixel loop: `spp` jittered samples per pixel, every
   dimension drawn in the fixed order whether or not anything uses it
   yet, and the same image at any thread count.
4. Write the `stratified` sampler.
5. Write the `halton` sampler.
6. Implement the six warps and their densities in the kit's `warp.cpp`,
   turn on the given `--warptest` (make your `warpTest()` in `rt.cpp`
   `return runWarpTest(o);`), and run the chi-square test:
   `tools/warptest.py all` must print PASS six times.
7. Implement the thin lens (`rtLens`).
8. Area lights, part 1: implement the light interface and mesh emitters;
   check the sampled points and PDFs numerically.
9. Area lights, part 2: connect next-event estimation to `cook`.
   `tools/area_light_test.py` must pass; inspect its RMSE trend too.
10. Area lights, part 3: implement sphere emitters, outside and inside.
11. Implement the environment light.
12. Implement glossy reflection, the coat, light rule, and caustic record.
13. Implement transmission, first clear glass, then frosted glass.
14. Implement ambient occlusion.
15. Render the test and study scenes under `cook`.
16. Run the convergence study (three samplers by default; `sobol` optional).
17. Render the hero scenes, check completeness, and write the report.

Take the formulas from the four technical handouts. They give them
exactly, down to which sample coordinate does what, so a renderer that
follows them matches the reference images:

- "Samplers": the interface, the dimension order, and the four samplers,
  with worked-out numbers;
- "Warps": the six mappings, the local frame, and the chi-square test;
- "Area Lights": the sampling interface, mesh and sphere emitters, the
  exact test, and the environment light;
- "Cook Shading Model": the shading equation, next-event estimation, the
  glossy lobe, ambient occlusion, and the trace log's format.

When the handout and one of them seem to disagree, follow the specific
one.

## The command line

The given `cmdline.cpp` parses HW3's options into `options`, as in HW2;
read them from there in your renderer:

    --integrator whitted|cook   which integrator renders (cook once RT_HOMEWORK is 3)
    --spp N                     samples per pixel, overriding the scene's rtPixelSamples()
    --sampler NAME              independent, stratified, halton, or sobol, overriding rtSampler()
    --seed S                    another set of random numbers (the references use seed 0, the default)
    --lens R                    the lens radius, overriding rtLens() (0: a pinhole)
    --light-samples N           N samples of every light per hit instead of one, spread on a
                                grid unless the sampler is independent
    --warptest WARP N [PARAM]   print N samples of a warp and exit (honors --sampler and --seed)

`--bsdftest`, `--env-sampling`, and `--ISstrategy` are HW4's: the given
code parses them, and a HW3 renderer ignores them. Everything from HW2
still works (`--width`, `--height`, `--threads`, `--out`, `--pfm`,
`--aov`, `--frames`, `--trace`, and the rest). Use `--width`, `--height`,
and `--spp` together to make any scene a quick render while you debug.

## The scenes

`scenes/2_Cook/` holds the new scenes, in four tiers. Each file's
comment says what the scene shows and tests, and `reference_renders/`
shows the picture:

- `1_TestScenes/`: `Stripes` and `Stripes_halton` (stripes down to half
  a pixel wide, the test of the stratified and halton samplers),
  `WarpView` (your warps' points on their domains),
  `ScatteringPebbles` (the thin lens at f/2 over forty thousand pebbles),
  `SphereLight` and `SphereLightInside`, `QuadLight`, `PerforatedLamp`
  (a lamp with holes in it), `ShadowRays`, `AmbientOcclusion`,
  `GlossyFurnace`, `MirrorLamp` (what a mirror takes from a small lamp
  and from a point light, marked by a red bead), `FrostedEarth` (a glass
  globe, clear over the oceans and frosted over the continents, with
  `FrostedEarthMovie`, the globe turning), and `Convergence`: each the
  check of one feature, in the order the workflow needs them.
- `2_StudyScenes/`: `FocusSweep` (eight frames, the focus moving down a
  row of spheres) and `ManyLights` (64 lamps, every one sampled at every
  hit).
- `3_HeroScenes/`: `MagicalClock`, `ChopperTitan`, and `Candy`, at 256 to
  1024 samples per pixel: long renders, hours each on eight cores.
- `4_ExtraCreditScenes/`: `MotionBlur`, `MotionBlurPan`, `1984`, and
  `MoleculeFastMovie`, the scenes of the motion blur extra credit; a
  renderer without it draws each at its first pose.

Of the scenes in `scenes/1_Whitted/`, `cook` renders again the ones it
changes, which have a `cook` reference: `Aliasing` (with `Aliasing_64`),
`DepthOfField` (three focus distances), `CornellBox` and
`CornellBoxSpheres`, `GlossySweep`, `Paperclip`, `Skylight`, and four
hero scenes: `BarcelonaPavilion`, `Chess`, `Crown`, and `FishyCat`. The
rest (the matte scenes, the mirror and glass scenes under point lights,
`Molecule` and `FunWithRefraction` with their movies, the benchmark
scenes, `Sponza`, `SanMiguel`, `WhiteLands`, and the others) would look
as they did under `whitted`, up to the antialiasing, so `gauchoRT all`
and the tiers leave them out under `cook` (`gauchoRT` with no argument
marks them "by name only under cook"). `gauchoRT all` renders every
scene with a `cook` reference; `gauchoRT test`, `study`, `hero`, and
`extra` render a tier of them. The Suggested Workflow says which scenes
match when.

## The tools and the studies

Run these four tools to check what no image comparison can, and put
their outputs in your write-up. They need Python 3 and NumPy;
`warptest.py` uses SciPy for the p-value when it is installed, and
`convergence.py` needs Matplotlib for its plot. Run them from the kit
folder; each finds your executable under `build/` or
`rust/target/release/` (`--rt PATH` names another).

    python tools/warptest.py all                 the chi-square test of the six warps,
                                                 with the observed, expected, and residual
                                                 histograms under warptest/
    python tools/warptest.py disk -n 256 --sampler stratified --scatter
                                                 256 points of a warp drawn, no test:
                                                 how the warp carries the strata
    python tools/warptest.py all --view          each warp's points as red spheres on its
                                                 domain, drawn by your renderer (the scene
                                                 WarpView, under cook), in warptest/
    python tools/area_light_test.py              QuadLight at 4 to 256 samples per pixel
                                                 against the exact image (Lambert's formula)
    python tools/light_samples.py                ShadowRays, a 3 x 3 grid of samples per
                                                 pixel against light samples, with times
    python tools/convergence.py                  Convergence with every sampler at 1 to 1024
                                                 samples per pixel, the error against the
                                                 count on log-log axes with fitted slopes

`convergence.py` measures against `Convergence_ref.pfm`, the converged
reference in `reference_renders/2_Cook/1_TestScenes/`.

## What to hand in

The handout has the list. In short, hand in:

- the kit folder with your changes and your README, HW2's `README.md`
  brought up to date for HW3, without `build/`, `target/`, `models/`,
  `textures/`, and `reference_renders/`, which we have;
- the images in `renders/2_Cook/` as your renderer writes them at the
  scenes' own settings: every scene with a `cook` reference, which is
  what `gauchoRT all` renders (of the extra-credit tier, the scenes of
  what you attempted), `FrostedEarthMovie`'s MP4, and the `FocusSweep`
  strip. Delete the `_diff.png` files `compare.py` leaves beside an image
  that was not identical;
- the write-up: the warp-test table, the area-light test's output, the
  light-samples grid, the convergence plot with its slopes and what they
  mean, a paragraph each on why next-event estimation is unbiased and on
  the glossy lobe's weight, your `whitted` and `cook` images of three
  scenes side by side with a paragraph on what `cook` adds, and what was
  hard.

In the README, give the platform, the build and run commands, the extra
credit you attempted and the scenes that show it, and your AI-use
statement.

## What stays frozen from HW2

Do not change these files:

- `include/rt.h` (this kit's) and `include/shaders.h`;
- every scene file (the three `_FIXED` copies once they have replaced
  HW2's files);
- the given files: `main.cpp`, `cmdline.*` (this kit's), `options.h`
  (but for `RT_HOMEWORK`), `transform.*`, `image.*` (this kit's),
  `noise.cpp`, `debug.*`, `timer.h`, `shaders.cpp`, `texture.*`, and their
  Rust twins (add `mod` lines to `main.rs` and nothing else).

Keep your `whitted` integrator rendering HW2's images as before: the
workflow's first check is that it does.

## AI tools

The handout's AI policy is HW2's: an AI tool may explain concepts, help
you debug, interpret errors, and write setup, build files, and scripts.
Implement the renderer yourself: the samplers, the warps, the thin lens,
the lights' sampling and next-event estimation, the environment light,
the glossy lobe, ambient occlusion, the `cook` integrator, and the
sampled pixel loop. The handout says what to hand in and how to
acknowledge what a tool did.

## Final checks and submission scope

Install Python dependencies with `python -m pip install numpy matplotlib pillow`.
SciPy is optional; `ffmpeg` is needed for the movie and focus strip.
Commands in the workflow that end in a backslash use Bash continuation;
in PowerShell remove the backslash and join the lines.

Before you hand in, run the final check of workflow step 17,
`compare.py --require-all`: it compares your images with the references
and names every reference image or movie you have not rendered. A plain
folder comparison checks only the images you have. `LightSamples.png` is
compared on its nine images, without the captions. The convergence CSV,
plot and gallery have no reference images: hand them in with the
write-up, and judge their numbers as step 16 says. `Convergence_ref` is
the kit's input, not a render of yours; leave out your PFMs, AOV images,
difference images and movie frames. Of the extra-credit scenes, hand in
only those of features you attempted.

The convergence tool runs `independent`, `stratified` and `halton` by
default; once you have written `sobol`, add
`--samplers independent,stratified,halton,sobol`. Read a linear pixel
with `python tools/read_pixel.py IMAGE.pfm X Y`, X and Y from the top
left.
