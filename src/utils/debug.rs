//! debug.rs: the debugging aids. Complete; nothing to change; the twin of
//! the C++ kit's debug.h and debug.cpp. --trace X Y logs what the renderer
//! does for one pixel, and the encoders below turn what a renderer knows
//! at a hit into the --aov images, so every renderer writes the same
//! pictures.

use std::cell::Cell;

use ultraviolet::Vec3;

thread_local! {
    /// This thread is on the traced pixel.
    static TRACE_ON: Cell<bool> = const { Cell::new(false) };
}

/// --trace X Y: the pixel loop calls trace_begin(px, py) before a pixel's
/// samples and trace_end() after them; in between, rt_trace!(...) prints
/// one line on stderr (println's format) when that pixel is the traced
/// one. Print at the points the Whitted Illumination handout names, in its
/// format, so your log diffs against the reference's (gauchoRT Spheres
/// --trace 320 240 with the reference library). Only the worker rendering
/// the traced pixel prints, so lines never interleave, and on every other
/// pixel an rt_trace! is one test of a thread-local flag, before its
/// arguments are even evaluated: free in the render loop.
pub fn trace_begin(px: i32, py: i32) {
    let o = crate::options::OPTIONS.read().expect("options");
    TRACE_ON.with(|t| t.set(o.trace_x >= 0 && px == o.trace_x && py == o.trace_y));
}

pub fn trace_end() {
    TRACE_ON.with(|t| t.set(false));
}

#[inline]
pub fn tracing() -> bool {
    TRACE_ON.with(|t| t.get())
}

#[macro_export]
macro_rules! rt_trace {
    ($($arg:tt)*) => {
        if $crate::debug::tracing() {
            eprintln!($($arg)*);
        }
    };
}

/* The --aov images, each a color from what the renderer has at the first
 * hit: the shading or geometric normal as 0.5 n + 0.5, depth as 1 / t,
 * (u, v, 0), the albedo as it is, the camera ray's cost (BVH nodes and
 * primitives tested) over 512, and the fraction of the scene's lights
 * whose shadow ray is unblocked (shadow). */
pub fn aov_uv_color(u: f32, v: f32) -> [f32; 3] {
    [u, v, 0.0]
}
pub fn aov_cost_color(cost: i32) -> [f32; 3] {
    let v = cost as f32 / 512.0;
    [v, v, v]
}
pub fn aov_shadow_color(unblocked: i32, lights: i32) -> [f32; 3] {
    let v = if lights > 0 {
        unblocked as f32 / lights as f32
    } else {
        0.0
    };
    [v, v, v]
}
