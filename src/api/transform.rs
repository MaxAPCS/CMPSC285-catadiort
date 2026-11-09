//! transform.rs: a 4 x 4 matrix, a small vector type, and the current
//! transformation (the CTM) with its stack and the rt* calls that edit it.
//! Complete in both kits; nothing to change; the twin of the C++ kit's
//! transform.h, transform.cpp and vec.h. RenderMan's and HW1's convention:
//! a point is a row vector, p' = p M, translation in the bottom row
//! (m[3][0..2]), "A then B" is A * B, and an RtMatrix copies straight in.
//! Normals go through the inverse transpose: n' = n (M^-1)^T. Each call
//! does CTM = M * CTM, so the call made last is applied to points first.
//! Angles are degrees. The motion blocks (rtMotionBegin/End) keep a pose
//! per time beside the CTM; a renderer with motion blur (HW3 extra credit)
//! interpolates between them.

#![allow(non_snake_case)] // the rt* names are the interface's

use std::sync::Mutex;
use ultraviolet::{Mat4, Vec3, Vec4};

const IDENTITY: Mat4 = Mat4::new(
    Vec4::new(1., 0., 0., 0.),
    Vec4::new(0., 1., 0., 0.),
    Vec4::new(0., 0., 1., 0.),
    Vec4::new(0., 0., 0., 1.),
);

/* the CTM ------------------------------------------------------------- */

/// The pose of the CTM at the time of an rtMotionBegin block made at this
/// level (rt.h, rtMotionBegin). A transform call inside a block multiplies
/// that pose alone, outside one the CTM and every pose; rtTransformBegin/End
/// save and restore them with the CTM.
#[derive(Clone, Copy)]
pub struct MotionKey {
    pub time: f32,
    pub m: Mat4,
}

#[derive(Clone)]
struct Level {
    m: Mat4,
    keys: Vec<MotionKey>,
}

struct Ctm {
    current: Level,
    editing: usize, /* the key of the open block, or -1 */
    stack: Vec<Level>,
}

static CTM: Mutex<Ctm> = Mutex::new(Ctm {
    current: Level {
        m: IDENTITY,
        keys: Vec::new(),
    },
    editing: usize::MAX,
    stack: Vec::new(),
});

#[inline]
fn apply(ctm: &mut Ctm, m: Mat4) {
    if ctm.editing != usize::MAX {
        let k = &mut ctm.current.keys[ctm.editing];
        k.m = k.m * m;
        return;
    }
    ctm.current.m = ctm.current.m * m;
    for k in ctm.current.keys.iter_mut() {
        k.m = k.m * m;
    }
}

/// The current transformation. rtBegin resets it to the identity with an
/// empty stack; rtTransformBegin/End push and pop; every other transform
/// call left-multiplies (CTM = M * CTM). The renderer reads it when a
/// primitive or a light is drawn: geometry is baked then. With motion poses
/// at this level (rtMotionBegin) it is the earliest pose, so a renderer
/// without motion blur draws everything at time 0, as rt.h says; one with
/// it reads ctm_keys().
pub(super) fn get() -> Mat4 {
    let c = CTM.lock().unwrap();
    let mut first: Option<&MotionKey> = None;
    for k in &c.current.keys {
        if first.map_or(true, |f| k.time < f.time) {
            first = Some(k);
        }
    }
    first.map_or(c.current.m, |k| k.m)
}

// /// The poses of the CTM at the times of the motion blocks at this level,
// /// in the order opened.
// pub fn ctm_keys() -> Vec<MotionKey> {
//     CTM.lock().unwrap().current.keys.clone()
// }

// /// The time of the block that is open, or -1 outside any block (rtCamera
// /// keys the camera by it).
// pub fn ctm_motion_time() -> f32 {
//     let c = CTM.lock().unwrap();
//     if c.editing != usize::MAX {
//         c.current.keys[c.editing].time
//     } else {
//         -1.0
//     }
// }

pub(super) fn clear() {
    let mut c = CTM.lock().unwrap();
    c.current = Level {
        m: Mat4::identity(),
        keys: Vec::new(),
    };
    c.editing = usize::MAX;
    c.stack.clear();
}

#[unsafe(no_mangle)]
pub extern "C" fn rtTransformBegin() {
    let mut c = CTM.lock().unwrap();
    if c.editing != usize::MAX {
        eprintln!("rtTransformBegin inside a motion block, ignored");
        return;
    }
    let level = c.current.clone();
    c.stack.push(level);
}

#[unsafe(no_mangle)]
pub extern "C" fn rtTransformEnd() {
    let mut c = CTM.lock().unwrap();
    c.current = c
        .stack
        .pop()
        .expect("rtTransformEnd without rtTransformBegin");
    c.editing = usize::MAX;
}

/// A block at a time that already has a pose at this level edits that
/// pose further; a new time starts from the CTM as it is now.
#[unsafe(no_mangle)]
pub extern "C" fn rtMotionBegin(time: f32) {
    let mut c = CTM.lock().unwrap();
    if c.editing != usize::MAX {
        eprintln!("rtMotionBegin inside a motion block, ignored");
        return;
    }
    if !(0.0..=1.0).contains(&time) {
        eprintln!("rtMotionBegin({}): the shutter is open from 0 to 1", time);
    }
    if let Some(i) = c.current.keys.iter().position(|k| k.time == time) {
        c.editing = i;
        return;
    }
    let m = c.current.m;
    c.current.keys.push(MotionKey { time, m });
    c.editing = c.current.keys.len() - 1;
}

#[unsafe(no_mangle)]
pub extern "C" fn rtMotionEnd() {
    let mut c = CTM.lock().unwrap();
    debug_assert!(c.editing != usize::MAX, "rtMotionEnd without rtMotionBegin");
    c.editing = usize::MAX;
}

#[unsafe(no_mangle)]
pub extern "C" fn rtTranslate(dx: f32, dy: f32, dz: f32) {
    apply(
        &mut CTM.lock().unwrap(),
        Mat4::from_translation(Vec3::new(dx, dy, dz)),
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn rtRotate(angle: f32, x: f32, y: f32, z: f32) {
    apply(
        &mut CTM.lock().unwrap(),
        Mat4::from_rotation_around(Vec3::new(x, y, z).into(), angle.to_radians()),
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn rtScale(sx: f32, sy: f32, sz: f32) {
    apply(
        &mut CTM.lock().unwrap(),
        Mat4::from_nonuniform_scale(Vec3::new(sx, sy, sz)),
    );
}

/// RtMatrix m: C passes the array as a pointer to its first row.
/// # Safety
/// `m` points at 16 floats, as every scene's RtMatrix does.
#[unsafe(no_mangle)]
pub extern "C" fn rtConcatTransform(m: &[f32; 16]) {
    apply(&mut CTM.lock().unwrap(), Mat4::from(m));
}
