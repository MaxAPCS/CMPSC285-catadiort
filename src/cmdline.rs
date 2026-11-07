//! cmdline.rs: the command line of gauchoRT (the README lists it): the
//! options, each overriding what the scene file set, read into OPTIONS
//! (options.rs); usage(), which lists them and the scenes; the scene tables
//! main.rs renders from, one per folder of scenes/ the build found
//! (build.rs; all and the tiers take the tables up to RT_HOMEWORK, a name
//! runs anywhere); and image_path, where rtWorldEnd writes. Complete;
//! nothing to change; the twin of the C++ kit's cmdline.cpp.

use std::ffi::{CStr, c_char};
use std::path::Path;

use crate::options::{OPTIONS, Options, RT_HOMEWORK};

/* scenes ----------------------------------------------------------------- */

/// One entry of the scene table (scenes.h's RtSceneEntry, same layout).
#[repr(C)]
struct RtSceneEntry {
    name: *const c_char, /* what you type: e.g., gauchoRT Spheres */
    function: Option<unsafe extern "C" fn()>, /* the scene's entry point */
    tier: *const c_char, /* "test", "benchmark", "study", "compare", "hero" or "extra" */
}

unsafe extern "C" {
    /// {name, function, tier} entries ending with a null name, one table per
    /// folder of scenes/; HW2's is always there, the others when build.rs
    /// found their folder.
    static RT_SCENES_HW2: RtSceneEntry;
    #[cfg(rt_have_scenes_hidden)]
    static RT_SCENES_HIDDEN: RtSceneEntry;
    #[cfg(rt_have_scenes_hw3)]
    static RT_SCENES_HW3: RtSceneEntry;
    #[cfg(rt_have_scenes_hw4)]
    static RT_SCENES_HW4: RtSceneEntry;
}

pub struct Scene {
    pub name: &'static str,
    pub tier: &'static str,
    pub function: unsafe extern "C" fn(),
}

/// A scene table, what to call it, the homework it belongs to (the hidden
/// benchmark scene counts as HW2's) and its folder.
pub struct SceneTable {
    pub scenes: Vec<Scene>,
    pub label: &'static str,
    pub hw: i32,
    pub folder: &'static str,
}

/// # Safety
/// `first` is the first entry of a table ending with a null name.
unsafe fn read_table(first: *const RtSceneEntry) -> Vec<Scene> {
    let mut out = Vec::new();
    unsafe {
        let mut entry = first;
        while !(*entry).name.is_null() {
            out.push(Scene {
                name: CStr::from_ptr((*entry).name).to_str().unwrap_or("?"),
                tier: CStr::from_ptr((*entry).tier).to_str().unwrap_or("?"),
                function: (*entry).function.expect("a scene without a function"),
            });
            entry = entry.add(1);
        }
    }
    out
}

/// One table per folder of scenes/ the build found.
pub fn tables() -> Vec<SceneTable> {
    let mut out = Vec::new();
    // SAFETY: each table is a C array ending with a null name.
    unsafe {
        out.push(SceneTable {
            scenes: read_table(std::ptr::addr_of!(RT_SCENES_HW2)),
            label: "HW2",
            hw: 2,
            folder: "scenes/1_Whitted",
        });
        #[cfg(rt_have_scenes_hidden)]
        out.push(SceneTable {
            scenes: read_table(std::ptr::addr_of!(RT_SCENES_HIDDEN)),
            label: "Hidden",
            hw: 2,
            folder: "scenes/hidden",
        });
        #[cfg(rt_have_scenes_hw3)]
        out.push(SceneTable {
            scenes: read_table(std::ptr::addr_of!(RT_SCENES_HW3)),
            label: "HW3",
            hw: 3,
            folder: "scenes/2_Cook",
        });
        #[cfg(rt_have_scenes_hw4)]
        out.push(SceneTable {
            scenes: read_table(std::ptr::addr_of!(RT_SCENES_HW4)),
            label: "HW4",
            hw: 4,
            folder: "scenes/3_Kajiya",
        });
    }
    out
}

/// Whether a scene renders a movie's numbered frames: its name ends in
/// "Movie" (scenes.h).
fn is_movie(name: &str) -> bool {
    name.ends_with("Movie")
}

/// Scene names are what you type, so no two entries may share one, in one
/// table or across two.
fn check_unique_names(tables: &[SceneTable]) {
    let mut names: Vec<(&str, &str)> = Vec::new();
    for t in tables {
        for s in &t.scenes {
            names.push((s.name, t.folder));
        }
    }
    names.sort();
    for pair in names.windows(2) {
        if pair[0].0 == pair[1].0 {
            fail(&format!(
                "scene '{}' is in {}/ and {}/: scene names must be unique",
                pair[0].0, pair[0].1, pair[1].1
            ));
        }
    }
}

/// Whether the scene argument selects this entry of a table of homework hw:
/// by name always; by "all", "stills" or its tier only when the table is
/// this homework's or an earlier one's (RT_HOMEWORK).
pub fn selected(scene: &str, s: &Scene, hw: i32) -> bool {
    if scene == s.name {
        return true;
    }
    if hw > RT_HOMEWORK {
        return false;
    }
    if scene == "stills" {
        return !is_movie(s.name);
    }
    scene == "all" || scene == s.tier
}

/* integrators and warps -------------------------------------------------- */

/// The integrators, the homework each arrives in, and the folder of
/// renders/ and reference_renders/ its images go to (numbered so the
/// folders sort in homework order). Each named for the paper it
/// implements: Whitted 1980; Cook, Porter and Carpenter 1984 (distribution
/// ray tracing); Kajiya 1986 (path tracing).
const INTEGRATORS: [(&str, i32, &str); 3] = [
    ("whitted", 2, "1_Whitted"),
    ("cook", 3, "2_Cook"),
    ("kajiya", 4, "3_Kajiya"),
];

/// The homework an integrator arrives in, or 0 if there is no such name.
fn integrator_homework(name: &str) -> i32 {
    INTEGRATORS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, hw, _)| *hw)
        .unwrap_or(0)
}

/// The warps of --warptest, and whether each takes a PARAM. The first six
/// are HW3's; ggx, the GGX distribution of visible normals, is HW4's.
const WARPS: [(&str, bool); 7] = [
    ("disk", false),
    ("cosine_hemisphere", false),
    ("uniform_sphere", false),
    ("uniform_cone", true),
    ("triangle", false),
    ("phong", true),
    ("ggx", true),
];

/* usage ------------------------------------------------------------------ */

/// The usage text and the list of scenes, on stderr.
pub fn usage(tables: &[SceneTable]) {
    eprint!(
        "usage: gauchoRT <scene> [number ...] [options]
                          (the numbers are the scene's own arguments, rtSceneArg)
       gauchoRT all | stills | test | benchmark | study | compare | hero | extra [options]
       gauchoRT --help       this text
                          (stills: all but the movies, the scenes named ...Movie)
       gauchoRT --warptest WARP N [PARAM]   (HW3) print N samples of a warp and exit;
                          WARP: disk, cosine_hemisphere, uniform_sphere, triangle,
                          uniform_cone COSTHETAMAX, phong EXPONENT, ggx ALPHA (HW4)
       gauchoRT --bsdftest [ARG ...]        (HW4) check the BSDFs and exit; the arguments,
                          up to the next option, are the renderer's to read
options, each overriding what the scene file set:
  --integrator whitted|cook|kajiya
                          which integrator renders (default: this homework's)
  --width W --height H    image size
  --spp N                 samples per pixel (whitted renders one whatever it says)
  --max-depth N           ray depth
  --ambient R G B         ka of the shading formula, overriding the scene's rtAmbient
                          (0 0 0: the lights alone; kajiya ignores it, as it does rtAmbient)
  --threads N             worker threads (default: all cores)
  --no-accel              brute force over every primitive (the timing table)
  --time-limit S          stop after S seconds of rendering; the LIMIT line says how
                          much was done and what the whole frame would take
  --progress              a line on stderr every two seconds while rendering: the
                          pixels done, the time, about how long is left
  --frames A-B            of a movie, render frames A to B only (from 0; N alone: one)
  --trace X Y             log what the renderer does for that pixel (debug.rs): its rays,
                          hits, lights and weights, on stderr
  --out PATH              write PATH.png (and PATH.pfm) instead of
                          renders/<integrator>/<name>
  --pfm                   also write the linear float image
  --format png|jpg        the image's file type (default png; jpg for sharing a picture)
  --tonemap none|reinhard|aces|filmic
                          the PNG's tone map (default: the scene's rtTonemap)
  --exposure EV           stops added before the tone map (default: the scene's)
  --lens R                the lens radius in meters, overriding the scene's rtLens
                          (0: a pinhole); the focus distance stays the scene's
  --aov NAME              a debug image instead of the beauty: raydir (the camera ray's
                          direction, before any hit: the check of the viewing geometry),
                          normal, ng, depth, uv, albedo, cost (the camera ray's BVH work
                          over 512), shadow (the fraction of the lights unblocked), ao
                          (HW3); the renderer refuses a name it does not have. Written as
                          renders/<integrator>/<name>_<aov> unless --out says otherwise
  --sampler independent|stratified|halton|sobol
                          (HW3) the sample sequence
  --seed N                (HW3) random seed; same seed, same image
  --light-samples N       (HW3) cook samples every light N times at each hit,
                          averaged (default 1)
  --env-sampling importance|uniform
                          (HW4) how kajiya samples the environment as a light: by its
                          brightness (default) or uniformly over the sphere
  --ISstrategy mis|light|bsdf
                          (HW4) kajiya's strategies for the light from the lights:
                          both weighted by MIS (mis, the default), or one alone
  --bvh sah|midpoint      which BVH builder (a renderer with one builder ignores it)
"
    );
    for table in tables {
        eprintln!(
            "{} scenes ({}/):{}",
            table.label,
            table.folder,
            if table.hw > RT_HOMEWORK {
                ", by name only until RT_HOMEWORK reaches this homework"
            } else {
                ""
            }
        );
        let mut tier = "";
        for s in &table.scenes {
            if s.tier != tier {
                tier = s.tier;
                eprintln!("  {}:", tier);
            }
            eprintln!(
                "    {}{}",
                s.name,
                if is_movie(s.name) {
                    " (movie: numbered frames)"
                } else {
                    ""
                }
            );
        }
    }
}

/* the options ------------------------------------------------------------ */

fn fail(message: &str) -> ! {
    eprintln!("{}", message);
    std::process::exit(1);
}

/// The one argument that follows an option, or exit with a message.
fn value_of<'a>(args: &'a [String], i: &mut usize, option: &str) -> &'a str {
    if *i + 1 >= args.len() {
        fail(&format!("{} needs a value", option));
    }
    *i += 1;
    &args[*i]
}

/// An integer option value of at least `least`, or exit with a message.
fn integer_at_least(option: &str, value: &str, least: i64) -> i32 {
    match value.parse::<i64>() {
        Ok(n) if n >= least => n as i32,
        _ => fail(&format!(
            "{} needs {}, not '{}'",
            option,
            if least > 0 {
                "a positive integer"
            } else {
                "a non-negative integer"
            },
            value
        )),
    }
}

fn positive_int(option: &str, value: &str) -> i32 {
    integer_at_least(option, value, 1)
}

fn number(option: &str, value: &str) -> f32 {
    match value.parse::<f64>() {
        Ok(x) => x as f32,
        Err(_) => fail(&format!("{} needs a number, not '{}'", option, value)),
    }
}

/// Fill OPTIONS from the command line and return the scene argument (None
/// if there was none).
fn read_options(args: &[String], tables: &[SceneTable]) -> Option<String> {
    let mut o = Options::new();
    let mut scene: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--integrator" => {
                o.integrator = value_of(args, &mut i, a).to_string();
                let hw = integrator_homework(&o.integrator);
                if hw == 0 {
                    fail(&format!(
                        "--integrator: whitted, cook or kajiya, not '{}'",
                        o.integrator
                    ));
                }
                if hw > RT_HOMEWORK {
                    fail(&format!(
                        "--integrator {} is HW{}'s integrator, and this renderer is at HW{}.\nWhen HW{} starts, set RT_HOMEWORK to {} in src/options.rs.",
                        o.integrator, hw, RT_HOMEWORK, hw, hw
                    ));
                }
            }
            "--width" => o.width = positive_int(a, value_of(args, &mut i, a)),
            "--height" => o.height = positive_int(a, value_of(args, &mut i, a)),
            "--spp" => o.spp = positive_int(a, value_of(args, &mut i, a)),
            "--max-depth" => o.max_depth = integer_at_least(a, value_of(args, &mut i, a), 0),
            "--threads" => o.threads = positive_int(a, value_of(args, &mut i, a)) as _,
            "--no-accel" => o.no_accel = true,
            "--ambient" => {
                let mut ambient = [0.; _];
                for k in 0..3 {
                    let v = value_of(args, &mut i, a);
                    match v.parse::<f64>() {
                        Ok(x) if x >= 0.0 => ambient[k] = x as f32,
                        _ => fail(&format!("--ambient: three numbers, 0 or more, not '{}'", v)),
                    }
                }
                o.ambient = Some(ambient);
            }
            "--time-limit" => {
                let v = value_of(args, &mut i, a);
                match v.parse::<f64>() {
                    Ok(s) if s > 0.0 => o.time_limit = s,
                    _ => fail(&format!("--time-limit: seconds, more than 0, not '{}'", v)),
                }
            }
            "--out" => o.out = value_of(args, &mut i, a).to_string(),
            "--pfm" => o.pfm = true,
            "--format" => {
                o.format = value_of(args, &mut i, a).to_string();
                if !matches!(o.format.as_str(), "png" | "jpg") {
                    fail(&format!("--format: png or jpg, not '{}'", o.format));
                }
            }
            "--tonemap" => {
                o.tonemap = value_of(args, &mut i, a).to_string();
                if !["none", "reinhard", "aces", "filmic"].contains(&o.tonemap.as_str()) {
                    fail(&format!(
                        "--tonemap: none, reinhard, aces or filmic, not '{}'",
                        o.tonemap
                    ));
                }
            }
            "--exposure" => {
                o.exposure = value_of(args, &mut i, a).parse::<f64>().unwrap_or(0.0) as f32;
                o.exposure_set = true;
            }
            "--aov" => {
                /* any name: the renderer knows which images it has and refuses
                 * another, so a later homework can add one without a change here */
                o.aov = value_of(args, &mut i, a).to_string();
                if o.aov.is_empty() || o.aov.starts_with('-') {
                    fail("--aov needs an image's name");
                }
            }
            "--sampler" => {
                o.sampler = value_of(args, &mut i, a).to_string();
                if !["independent", "stratified", "halton", "sobol"].contains(&o.sampler.as_str()) {
                    fail(&format!(
                        "--sampler: independent, stratified, halton or sobol, not '{}'",
                        o.sampler
                    ));
                }
            }
            "--seed" => {
                let v = value_of(args, &mut i, a);
                match v.parse::<u64>() {
                    Ok(n) => o.seed = n,
                    Err(_) => fail(&format!("--seed needs a non-negative integer, not '{}'", v)),
                }
            }
            "--light-samples" => {
                o.light_samples = value_of(args, &mut i, a).parse::<i32>().unwrap_or(0);
                if o.light_samples < 1 {
                    fail("--light-samples needs a positive integer");
                }
            }
            "--warptest" => {
                o.warp = value_of(args, &mut i, a).to_string();
                let Some((_, param)) = WARPS.iter().find(|(n, _)| *n == o.warp) else {
                    fail(&format!(
                        "--warptest: disk, cosine_hemisphere, uniform_sphere, uniform_cone, triangle, phong or ggx, not '{}'",
                        o.warp
                    ));
                };
                o.warp_count = positive_int(a, value_of(args, &mut i, a));
                if *param {
                    o.warp_param = number(a, value_of(args, &mut i, a));
                }
            }
            "--bsdftest" => {
                /* Everything up to the next option is the renderer's to read,
                 * so the BSDF tests can take what HW4 needs without a change
                 * here. */
                o.bsdf_test = true;
                while i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    i += 1;
                    o.bsdf_test_args.push(args[i].clone());
                }
            }
            "--ISstrategy" => {
                o.is_strategy = value_of(args, &mut i, a).to_string();
                if !["mis", "light", "bsdf"].contains(&o.is_strategy.as_str()) {
                    fail(&format!(
                        "--ISstrategy: mis, light or bsdf, not '{}'",
                        o.is_strategy
                    ));
                }
            }
            "--env-sampling" => {
                o.env_sampling = value_of(args, &mut i, a).to_string();
                if !["importance", "uniform"].contains(&o.env_sampling.as_str()) {
                    fail(&format!(
                        "--env-sampling: importance or uniform, not '{}'",
                        o.env_sampling
                    ));
                }
            }
            "--bvh" => {
                o.bvh = value_of(args, &mut i, a).to_string();
                if !matches!(o.bvh.as_str(), "sah" | "midpoint") {
                    fail(&format!("--bvh: sah or midpoint, not '{}'", o.bvh));
                }
            }
            "--help" | "-h" => {
                usage(tables);
                std::process::exit(0);
            }
            "--progress" => o.progress = true,
            "--frames" => {
                let v = value_of(args, &mut i, a);
                let (first, last) = match v.split_once('-') {
                    Some((f, l)) => (f.parse::<i32>().ok(), l.parse::<i32>().ok()),
                    None => (v.parse::<i32>().ok(), v.parse::<i32>().ok()),
                };
                match (first, last) {
                    (Some(f), Some(l)) if f >= 0 && l >= f => {
                        o.frame_first = f;
                        o.frame_last = l;
                    }
                    _ => fail(&format!(
                        "--frames: a frame number N or a range A-B, from 0, not '{}'",
                        v
                    )),
                }
            }
            "--trace" => {
                o.trace_x = integer_at_least(a, value_of(args, &mut i, a), 0);
                o.trace_y = integer_at_least(a, value_of(args, &mut i, a), 0);
            }
            "--lens" => {
                o.lens = number(a, value_of(args, &mut i, a));
                if o.lens < 0.0 {
                    fail("--lens needs a radius of 0 or more");
                }
            }
            _ if a.starts_with("--") => {
                eprintln!("unknown option '{}'", a);
                usage(tables);
                std::process::exit(1);
            }
            _ => {
                if let Some(s) = &scene {
                    /* a number after the scene's name is the scene's own argument (rtSceneArg) */
                    match a.parse::<f64>() {
                        Ok(v) => o.scene_args.push(v as f32),
                        Err(_) => fail(&format!(
                            "one scene (or tier) at a time: '{}' and '{}'",
                            s, a
                        )),
                    }
                } else {
                    scene = Some(a.to_string());
                }
            }
        }
        i += 1;
    }
    *OPTIONS.write().expect("options") = o;
    scene
}

/// Read the command line into OPTIONS and return the scene argument: a
/// scene's name, a tier, "all" or "stills". Everything that does not end in
/// a render is settled in here and exits: a bad option, --help, no scene
/// argument, and the tests --warptest (HW3) and --bsdftest (HW4).
pub fn parse_command_line(args: &[String], tables: &[SceneTable]) -> String {
    check_unique_names(tables);
    let scene = read_options(args, tables);

    /* A test instead of a render: --warptest WARP N [PARAM] (HW3) or
     * --bsdftest [ARG ...] (HW4), one at a time and with no scene. The
     * renderer defines both (rt.rs); its exit code is ours. */
    let o = crate::options::options();
    if !o.warp.is_empty() || o.bsdf_test {
        let test = if o.bsdf_test {
            "--bsdftest"
        } else {
            "--warptest"
        };
        if !o.warp.is_empty() && o.bsdf_test {
            fail("--warptest and --bsdftest: one test at a time");
        }
        if let Some(s) = &scene {
            fail(&format!("{} renders nothing: drop '{}'", test, s));
        }
        std::process::exit(if o.bsdf_test {
            crate::api::bsdf_test(&o)
        } else {
            crate::api::warp_test(&o)
        });
    }

    let Some(scene) = scene else {
        usage(tables);
        std::process::exit(1);
    };
    scene
}

/* where an image goes ---------------------------------------------------- */

/// Where rtWorldEnd writes an image, without extension: --out as given,
/// else renders/<integrator>/<display>, <integrator> the folder 1_Whitted,
/// 2_Cook or 3_Kajiya and <display> the rtDisplay name, with _<aov>
/// appended for a debug image, _light or _bsdf for --ISstrategy, and
/// _uniform for --env-sampling uniform, so a variant never overwrites the
/// default. Makes the folder, so a renderer only ever has to write a file.
pub fn image_path(display: &str) -> String {
    let o = OPTIONS.read().expect("options");
    let mut path = o.out.clone();
    if path.is_empty() {
        let folder = INTEGRATORS
            .iter()
            .find(|(n, _, _)| *n == o.integrator)
            .map(|(_, _, f)| *f)
            .unwrap_or(o.integrator.as_str());
        path = format!("renders/{}/{}", folder, display);
        if !o.aov.is_empty() {
            path = format!("{}_{}", path, o.aov);
        }
        if o.is_strategy != "mis" {
            path = format!("{}_{}", path, o.is_strategy);
        }
        if o.env_sampling == "uniform" {
            path = format!("{}_uniform", path);
        }
    }
    if let Some(parent) = Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    path
}
