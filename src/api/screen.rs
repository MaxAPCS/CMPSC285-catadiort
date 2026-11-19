use core::num::NonZeroUsize;
use std::{
    ffi::{CStr, c_char},
    sync::Mutex,
};
use ultraviolet::Vec3;

use super::*;
use crate::engine::{Screen, integrators};
use crate::options::{Integrator, OPTIONS, Options};
use crate::utils::{
    image::write_image,
    texture,
    timer::{RenderBudget, Timer},
};
use crate::{AOV, options::Sampler, utils::samplers};

/// Maximum Concurrent Rays
const RAY_CONCURRENCY: u32 = 2u32.pow(22);

/// The graphics state. rtBegin resets it to the defaults listed at
/// rtBegin. Options and attributes persist across rtWorldBegin/End pairs;
/// the world's contents (lights, primitives) are cleared at rtWorldBegin, so a
/// scene that renders several images builds each world afresh. The
/// finished renderer keeps more here: the lights, the primitives, the
/// textures, the BVH.
pub struct ScreenState {
    pub display: String, /* rtDisplay's name, without extension */
    pub res: (u16, u16),
    pub samples: u32,

    pub max_depth: u16,
    pub ambient: RtColor,
    pub background: RtColor,
    pub environment: i32, /* rtEnvironment's map, a texture handle (-1: none, the background shows) */
    pub environment_scale: f32,
    pub environment_rotate_y: f32,
    pub eye: RtVector,
    pub lookat: RtVector,
    pub up: RtVector,
    pub fovy: f32,
    pub tonemap: String, /* rtTonemap: the PNG's tone map and exposure (stops) */
    pub tint: RtColor,
    pub vignette: f32,
    pub exposure: f32,

    /* The [HW3] calls, stored now and read by cook (HW3): whitted renders
     * through a pinhole, one ray per pixel center, with plain ambient. */
    pub lens_radius: f32,
    pub focus_distance: f32,
    pub sampler: Sampler,
    pub ao_distance: f32,
    pub has_portal: bool, /* rtEnvironmentPortal's quad, world space; sampling the environment through it is
                          HW3 extra credit, and nothing reads it until then */
    pub portal: [[f32; 3]; 4],
}

impl ScreenState {
    pub const fn new() -> ScreenState {
        ScreenState {
            res: (640, 480),
            samples: 1,
            max_depth: 8,
            ambient: Vec3::broadcast(0.),
            background: Vec3::broadcast(0.),
            environment: -1,
            environment_scale: 1.0,
            environment_rotate_y: 0.0,
            eye: Vec3::broadcast(0.),
            lookat: Vec3::new(0.0, 0.0, -1.0),
            up: Vec3::new(0.0, 1.0, 0.0),
            fovy: 45.0,
            display: String::new(),
            tonemap: String::new(),
            tint: Vec3::broadcast(1.),
            vignette: 0.,
            exposure: 0.0,
            lens_radius: 0.0,
            focus_distance: 1.0,
            sampler: Sampler::Center,
            ao_distance: 0.0,
            has_portal: false,
            portal: [[0.0; 3]; 4],
        }
    }
}

/// Only the main thread touches the state through the rt* calls; the pixel
/// loop's workers read what rtWorldEnd hands them.
pub static STATE: Mutex<ScreenState> = Mutex::new(ScreenState::new());

#[inline]
fn state() -> std::sync::MutexGuard<'static, ScreenState> {
    STATE.lock().expect("state")
}

/// A C string argument as a Rust String ("" for a null pointer).
/// # Safety
/// `s` is null or a NUL-terminated string.
#[inline]
fn text(s: *const c_char) -> String {
    if s.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(s) }.to_string_lossy().into_owned()
    }
}

/// The worker count: --threads, or every core.
pub fn thread_count(o: &Options) -> NonZeroUsize {
    if let Some(t) = NonZeroUsize::new(o.threads) {
        return t;
    }
    std::thread::available_parallelism().unwrap_or(unsafe { NonZeroUsize::new_unchecked(1) })
}

/* session, options ------------------------------------------------------ */

/// Start a session and reset everything to these defaults:
/// 640 x 480, 1 spp, max depth 8, no ambient, a black background and no
/// environment, the camera at the origin looking down -z with +y up and
/// fovy 45, a pinhole, the independent sampler, no ambient occlusion, the
/// default material, no shaders, the CTM the identity with an empty stack.
/// rtEnd frees the textures and the geometry.
#[unsafe(no_mangle)]
pub extern "C" fn rtBegin() {
    let mut s = state();
    *s = ScreenState::new();
    s.display = "out".to_string();
    s.tonemap = "none".to_string();
    s.sampler = Sampler::Independent;
    transform::clear();
    texture::clear();
}

#[unsafe(no_mangle)]
pub extern "C" fn rtEnd() {
    *state() = ScreenState::new();
    texture::clear();
}

/// The image size in pixels (--width and --height override it).
#[unsafe(no_mangle)]
pub extern "C" fn rtFormat(xres: RtInt, yres: RtInt) {
    let mut s = state();
    s.res = (xres.max(1) as _, yres.max(1) as _)
}

/// The output name without extension: rtWorldEnd writes
/// renders/<integrator>/<name>.png, plus the .pfm with --pfm (image_path in
/// cmdline.rs; --out replaces the whole path).
/// # Safety
/// `name` is null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rtDisplay(name: *const c_char) {
    let n = text(name);
    state().display = if n.is_empty() { "out".to_string() } else { n };
}

/// The scene's own arguments: the numbers after its name on the command
/// line (cmdline.rs), or the default where none was given.
#[unsafe(no_mangle)]
pub extern "C" fn rtSceneArg(index: RtInt, default_value: RtFloat) -> RtFloat {
    let o = crate::options::OPTIONS.read().expect("options");
    if index >= 0 && (index as usize) < o.scene_args.len() {
        o.scene_args[index as usize]
    } else {
        default_value
    }
}

/// The PNG's look: stored here, applied by write_image (image.rs, given
/// complete); --tonemap and --exposure override it.
/// # Safety
/// `op` is null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rtTonemap(op: *const c_char, exposure: RtFloat) {
    let mut s = state();
    let t = text(op);
    s.tonemap = if t.is_empty() { "none".to_string() } else { t };
    s.exposure = exposure;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rtColorGrade(tint: &RtColor, vignette: RtFloat) {
    let mut s = state();
    s.tint = *tint;
    s.vignette = vignette;
}

/// Samples per pixel. whitted renders one whatever it says; cook and
/// kajiya (HW3, HW4) use it.
#[unsafe(no_mangle)]
pub extern "C" fn rtPixelSamples(spp: RtInt) {
    state().samples = spp.max(1).isqrt() as _;
}

/// The deepest secondary ray: a ray at depth d spawns children only if
/// d < max_depth (Whitted Illumination handout).
#[unsafe(no_mangle)]
pub extern "C" fn rtMaxDepth(depth: RtInt) {
    state().max_depth = depth.max(0) as _;
}

/// The ambient term: ka * albedo at every lit point (Whitted Illumination
/// handout); cook scales it by occlusion, kajiya ignores it.
/// # Safety
/// `ka` points at three floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rtAmbient(ka: &RtVector) {
    state().ambient = *ka;
}

/// The radiance a ray that leaves the scene returns, at any depth, when no
/// environment map is set.
/// # Safety
/// `radiance` points at three floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rtBackground(radiance: &RtColor) {
    state().background = *radiance;
}

/// A latitude-longitude map in place of the background (Workflow, step 13):
/// loaded now as a texture (PFM or HDR, linear; an 8-bit image is decoded
/// from sRGB), looked up at render time by the direction turned by -rotateY
/// about +y (the map turns by +rotateY, as rtRotate turns a textured
/// sphere), at the sphere's u and v of that direction. NULL removes it; a
/// file that does not load leaves the background showing.
/// # Safety
/// `file` is null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rtEnvironment(file: *const c_char, scale: RtFloat, rotate_y: RtFloat) {
    let mut s = state();
    s.environment = if file.is_null() {
        -1
    } else {
        unsafe { rtMakeTexture(file, 1) }
    };
    s.environment_scale = scale;
    s.environment_rotate_y = rotate_y;
}

/// The camera in the world: eye, look-at, up and the full vertical field of
/// view in degrees (Viewing Geometry handout); the CTM does not apply. Stored
/// here; rtWorldEnd builds the camera's frame from it (Workflow, step 5).
/// ([HW3, extra credit]: inside a motion block the call keys the camera at the block's
/// time, ctm_motion_time(); store that too. A renderer with motion blur
/// reads it; nothing does until then.)
/// # Safety
/// The three pointers point at three floats each.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rtCamera(eye: &Vec3, lookat: &Vec3, up: &Vec3, fovy: RtFloat) {
    let mut s = state();
    s.eye = *eye;
    s.lookat = *lookat;
    s.up = *up;
    s.fovy = fovy;
}

/* textures --------------------------------------------------------------- */

/// The texture table is given (texture.rs): texture::load reads the file
/// through read_image (image.rs: PNG, JPEG, BMP, TGA, HDR and PFM as linear
/// floats, the 8-bit color channels decoded from sRGB when asked), trying
/// the path as given and then under textures/, and returns a handle (-1
/// with a message when it cannot); handles live until rtEnd.
/// # Safety
/// `file` is null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rtMakeTexture(file: *const c_char, srgb: RtInt) -> RtInt {
    texture::load(&text(file), srgb != 0)
}

/// Bilinear lookup with repeat wrap; s left to right, t top to bottom (t = 0
/// is the top row); alpha 1 when the image has none. Given (texture.rs).
/// # Safety
/// `rgba` points at four floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rtTexture(handle: RtInt, s: RtFloat, t: RtFloat, rgba: *mut RtFloat) {
    let v = texture::lookup(handle, s, t);
    unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), rgba, 4) };
}

/* [HW3] calls ------------------------------------------------------------- */

/// Stored in the graphics state; whitted ignores all four. cook (HW3)
/// reads the lens, the sampler and the AO distance; the portal is HW3 extra
/// credit, read by no integrator yet.
#[unsafe(no_mangle)]
pub extern "C" fn rtLens(radius: RtFloat, focus_distance: RtFloat) {
    let mut s = state();
    s.lens_radius = radius.max(0.0);
    s.focus_distance = if focus_distance > 0.0 {
        focus_distance
    } else {
        1.0
    };
}

/// # Safety
/// `name` is null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rtSampler(name: *const c_char) {
    let n = text(name);
    let mut s = state();
    let sampstr = n.as_str();
    match sampstr.try_into() {
        Ok(sampler) => s.sampler = sampler,
        Err(_) => eprintln!(
            "{}: rtSampler(\"{}\"): not a sampler, keeping {}",
            OPTIONS.read().unwrap().scene,
            sampstr,
            s.sampler.to_string()
        ),
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn rtAmbientOcclusion(max_distance: RtFloat) {
    state().ao_distance = max_distance.max(0.0);
}

/// # Safety
/// `corners` is null or points at four RtPoints.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rtEnvironmentPortal(corners: *const RtPoint) {
    let mut s = state();
    s.has_portal = !corners.is_null();
    if !corners.is_null() {
        let m = transform::get();
        for i in 0..4 {
            s.portal[i] = m
                .transform_point3(Vec3::from(unsafe { *corners.add(i) }))
                .into();
        }
    }
}

/* world ------------------------------------------------------------------ */

/// rtWorldBegin starts a world: the lights and primitives drawn from here to
/// rtWorldEnd are what the image shows. The options and attributes set so
/// far carry over. Returns whether --frames wants this frame; a movie skips
/// the draw calls of one it does not (rt.h).
#[unsafe(no_mangle)]
pub extern "C" fn rtWorldBegin(frame: RtInt) -> RtInt {
    world::clear();
    let o = OPTIONS.read().unwrap();
    (o.frame_first < 0 || (frame >= o.frame_first && frame <= o.frame_last)) as RtInt
}

#[unsafe(no_mangle)]
pub extern "C" fn rtWorldEnd() {
    let o = OPTIONS.read().unwrap();
    let mut s = state();
    if let Some(width) = o.width {
        s.res.0 = width.into();
    }
    if let Some(height) = o.height {
        s.res.1 = height.into();
    }
    if let Some(spp) = o.spp {
        s.samples = spp.isqrt().get() as _;
    }
    if o.max_depth >= 0 {
        s.max_depth = o.max_depth as _;
    }
    if let Some(ambient) = o.ambient {
        s.ambient = ambient.into();
    }
    if let Some(sampler) = o.sampler {
        s.sampler = sampler;
    }
    if o.lens >= 0. {
        s.lens_radius = o.lens;
    }
    let seed = o.seed;

    let total = Timer::new();
    let (build_time, render_time, budget, frame) = {
        let mut world = world::take();
        world.ambientcolor = s.ambient.into();
        world.environmentcolor = s.background.into();
        world.environmenttex = s.environment;
        world.environmentscale = s.environment_scale;
        world.environmentrotation = s.environment_rotate_y.to_radians();
        world.finish();

        let mut frame = vec![Vec3::zero(); s.res.0 as usize * s.res.1 as usize].into_boxed_slice();

        let halfheight = (s.fovy.to_radians() / 2.).tan();

        let tofwd = (s.lookat - s.eye).normalized();
        let up = tofwd.cross(s.up).cross(tofwd).normalized();
        let totop = halfheight * up;
        let torgh = halfheight * (s.res.0 as f32 / s.res.1 as f32) * Vec3::cross(&tofwd, up);
        let res: (u32, u32, u32) = (s.res.0 as _, s.res.1 as _, s.samples as _);

        let build_time = total.seconds();
        let render = Timer::new();
        let budget = RenderBudget::new(o.time_limit); /* --time-limit: the clock starts here (timer.rs) */

        let step = RAY_CONCURRENCY / (res.0 * res.2 * res.2);
        for i in (0..res.1).step_by(step as usize) {
            let hslice = i..((i + step) as u32).min(res.1);
            let pixels = hslice.len() as u64 * (res.0 as u64);

            let mut screen = match s.sampler {
                Sampler::Center => Screen::create::<samplers::Center>(
                    frame,
                    s.eye,
                    tofwd,
                    totop,
                    torgh,
                    s.focus_distance,
                    s.lens_radius,
                    res,
                    hslice,
                    seed,
                ),
                Sampler::Independent => Screen::create::<samplers::Independent>(
                    frame,
                    s.eye,
                    tofwd,
                    totop,
                    torgh,
                    s.focus_distance,
                    s.lens_radius,
                    res,
                    hslice,
                    seed,
                ),
                Sampler::Stratified => Screen::create::<samplers::Stratified>(
                    frame,
                    s.eye,
                    tofwd,
                    totop,
                    torgh,
                    s.focus_distance,
                    s.lens_radius,
                    res,
                    hslice,
                    seed,
                ),
                Sampler::Halton => Screen::create::<samplers::Halton>(
                    frame,
                    s.eye,
                    tofwd,
                    totop,
                    torgh,
                    s.focus_distance,
                    s.lens_radius,
                    res,
                    hslice,
                    seed,
                ),
                Sampler::Sobol => Screen::create::<samplers::Sobol>(
                    frame,
                    s.eye,
                    tofwd,
                    totop,
                    torgh,
                    s.focus_distance,
                    s.lens_radius,
                    res,
                    hslice,
                    seed,
                ),
            };

            match o.integrator {
                Integrator::Whitted => screen.process::<integrators::Whitted>(&world, s.max_depth),
                Integrator::Cook => screen.process::<integrators::Cook>(&world, s.max_depth),
                _ => unimplemented!(),
            };

            frame = screen.write();
            budget.done(pixels);
            if budget.expired() {
                break;
            }
        }

        let render_time = render.seconds();
        budget.finish();
        (build_time, render_time, budget, frame)
    };

    /* The scene's tone map and exposure, unless the command line says
     * otherwise; a debug image (--aov) is written as it is. */
    let tonemap = if AOV {
        "none".to_string()
    } else if o.tonemap.is_empty() {
        s.tonemap.clone()
    } else {
        o.tonemap.clone()
    };
    let exposure = if AOV {
        0.0
    } else if o.exposure_set {
        o.exposure
    } else {
        s.exposure
    };
    let tint = if AOV { Vec3::one() } else { s.tint };
    let vignette = if AOV { 0. } else { s.vignette };
    write_image(
        &crate::cmdline::image_path(&s.display),
        s.res.0 as _,
        s.res.1 as _,
        frame,
        o.pfm,
        &tonemap,
        exposure,
        &o.format,
        tint,
        vignette,
    );

    let scene_name = if !o.scene.is_empty() {
        o.scene.clone()
    } else {
        s.display.clone()
    };
    println!(
        "TIME {} build={:.3} render={:.3} total={:.3} threads={}",
        scene_name,
        build_time,
        render_time,
        total.seconds(),
        thread_count(&o)
    );
    budget.report(
        &scene_name,
        render_time,
        s.res.0 as u64 * s.res.1 as u64,
        (s.samples * s.samples) as _,
    );
}
