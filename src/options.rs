//! options.rs: the options the command line sets (cmdline.rs) and the renderer
//! reads at rtWorldEnd, and RT_HOMEWORK. Every option overrides what the
//! scene file set, so the benchmark and the studies can be scripted without
//! editing scenes; the "unset" value means "as the scene said". Complete;
//! the twin of the C++ kit's options.h. Nothing to change here but
//! RT_HOMEWORK, once per homework.

#![allow(dead_code)] // given complete: your renderer uses the rest as it grows

use std::{
    num::NonZeroU16,
    sync::{LazyLock, RwLock},
};

/// Which homework this renderer is at: 2, 3 or 4. Set it when a homework
/// starts. It picks the default integrator (whitted, cook, kajiya), which
/// integrators --integrator accepts (this homework's and the earlier
/// ones'), and which scene tables `gauchoRT all` and the tiers include.
/// A scene name renders any scene the build has.
pub const RT_HOMEWORK: i32 = 2;

/// The integrator of homework RT_HOMEWORK: 2 whitted, 3 cook, 4 kajiya.
pub fn homework_integrator() -> &'static str {
    match RT_HOMEWORK {
        4.. => "kajiya",
        3 => "cook",
        _ => "whitted",
    }
}

/// The options, each overriding what the scene file set; "" or a negative
/// number means "the scene's". cmdline.rs fills them before any scene runs;
/// read them through options() (a copy) or OPTIONS.read().
#[derive(Clone, Debug)]
pub struct Options {
    pub integrator: String, /* --integrator whitted|cook|kajiya; this homework's unless given */
    pub width: Option<NonZeroU16>, /* --width W --height H; 0: the scene's rtFormat */
    pub height: Option<NonZeroU16>,
    pub spp: Option<NonZeroU16>, /* --spp N; 0: the scene's rtPixelSamples */
    pub max_depth: i32, /* --max-depth N (0: no secondary rays); -1: the scene's rtMaxDepth */
    pub ambient: Option<[f32; 3]>, /* --ambient R G B: ka of the shading formula, overriding the scene's rtAmbient */
    pub threads: usize,            /* --threads N; 0: all cores */
    pub no_accel: bool,            /* --no-accel: a linear scan over every primitive */
    pub time_limit: f64, /* --time-limit S: stop handing out tiles after S seconds of rendering and print the
                         LIMIT line (HW2 handout: timing); 0: render everything */
    pub out: String, /* --out PATH: write PATH.png (and .pfm); "": renders/<integrator>/<name> */
    pub pfm: bool,   /* --pfm: also write the linear float image */
    pub format: String, /* --format png|jpg: the image's file type; the PFM of --pfm is separate */
    pub tonemap: String, /* --tonemap none|reinhard|aces|filmic: the PNG's tone map; "": the scene's rtTonemap */
    pub exposure: f32,   /* --exposure EV: stops added before the tone map */
    pub exposure_set: bool, /*   whether --exposure was given; else the scene's */
    pub aov: String, /* --aov NAME: a debug image instead of the beauty, by name; the renderer knows
                     which it has (HW2: raydir, normal, ng, depth, uv, albedo, cost, shadow; HW3 adds
                     ao) and refuses another; "": the beauty image */
    pub trace_x: u32, /* --trace X Y: the pixel whose rays the renderer logs (debug.rs); -1: none */
    pub trace_y: u32,
    pub progress: bool, /* --progress: a line on stderr every two seconds while rendering (timer.rs) */
    pub frame_first: i32, /* --frames A-B: render frames A to B of a movie only (the worlds since rtBegin,
                          from 0); -1: every frame */
    pub frame_last: i32,
    pub lens: f32, /* --lens R: the lens radius, overriding the scene's rtLens (0: a pinhole); -1: the scene's */
    pub sampler: String, /* --sampler independent|stratified|halton|sobol (HW3); "": the scene's rtSampler */
    pub seed: u64,       /* --seed N (HW3): the same seed gives the same image */
    pub light_samples: i32, /* --light-samples N (HW3): cook samples every light N times at a hit */
    pub warp: String,    /* --warptest WARP N [PARAM] (HW3): the warp, "" when not testing one */
    pub warp_count: i32, /*   how many samples to print */
    pub warp_param: f32, /*   uniform_cone's cosThetaMax, phong's exponent or ggx's alpha; 0 for the others */
    pub bsdf_test: bool, /* --bsdftest [ARG ...] (HW4): check the BSDFs instead of rendering */
    pub bsdf_test_args: Vec<String>, /* the arguments after it, up to the next option */
    pub scene_args: Vec<f32>, /* the numbers after the scene's name: rtSceneArg reads them */
    pub env_sampling: String, /* --env-sampling importance|uniform (HW4): how kajiya samples the environment as
                              a light; whitted and cook ignore it */
    pub is_strategy: String, /* --ISstrategy mis|light|bsdf (HW4): kajiya's strategies for light that arrives from
                             the lights, both weighted by MIS or one alone; whitted and cook ignore it */
    pub bvh: String, /* --bvh sah|midpoint: which builder; a renderer with one ignores it (Workflow, step 14) */
    pub scene: String, /* the name of the scene being run, for the TIME line */
}

impl Options {
    pub fn new() -> Options {
        Options {
            integrator: homework_integrator().to_string(),
            width: None,
            height: None,
            spp: None,
            max_depth: -1,
            ambient: None,
            threads: 0,
            no_accel: false,
            time_limit: 0.0,
            out: String::new(),
            pfm: false,
            format: "png".to_string(),
            tonemap: String::new(),
            exposure: 0.0,
            exposure_set: false,
            aov: String::new(),
            trace_x: u32::MAX,
            trace_y: u32::MAX,
            progress: false,
            frame_first: -1,
            frame_last: -1,
            lens: -1.0,
            sampler: String::new(),
            seed: 0,
            light_samples: 1,
            warp: String::new(),
            warp_count: 0,
            warp_param: 0.0,
            bsdf_test: false,
            bsdf_test_args: Vec::new(),
            scene_args: Vec::new(),
            env_sampling: "importance".to_string(),
            is_strategy: "mis".to_string(),
            bvh: "sah".to_string(),
            scene: String::new(),
        }
    }
}

impl Default for Options {
    fn default() -> Options {
        Options::new()
    }
}

/// The options, filled by cmdline.rs before any scene runs (the simple main
/// of main.rs sets a few by hand instead).
pub static OPTIONS: LazyLock<RwLock<Options>> = LazyLock::new(|| RwLock::new(Options::new()));

/// A copy of the options, for a renderer that reads them once at rtWorldEnd.
pub fn options() -> Options {
    OPTIONS.read().expect("options").clone()
}
