use ultraviolet::{Vec3, Vec3x4};
use wide::{CmpLt, f32x4};

use super::DEPTH_COMPLEXITY;
use super::object::{Primitive, Sphere};
use super::util::f32x4_NAN;

pub(super) struct Rayx4<'a> {
    pub pos: Vec3x4,
    pub dir: Vec3x4,

    hits: smallvec::SmallVec<[Hitsx4<'a, Sphere>; DEPTH_COMPLEXITY]>,
}

impl<'a> Rayx4<'a> {
    #[inline]
    pub fn new(pos: Vec3x4, dir: Vec3x4) -> Self {
        Self {
            pos,
            dir,
            hits: smallvec::SmallVec::new(),
        }
    }

    #[inline]
    /// Insert hits on one object to every lane
    pub fn intersect(&mut self, primitive: &'a Sphere, depth: u16) {
        let t = primitive.intersectx4(self, depth);
        if t.is_nan().all() {
            return;
        };
        self.hits.push(Hitsx4 {
            t,
            obj: [primitive; _],
            uv: (f32x4_NAN, f32x4_NAN),
        });
    }

    #[inline]
    /// Collapse hits combining each lane's by distance
    pub fn collapse(&mut self) {
        let Some(mut hit) = self.hits.pop() else {
            return;
        };

        #[cfg(debug_assertions)]
        if self.hits.len() + 1 > DEPTH_COMPLEXITY {
            println!(
                "ray: collapse hit count estimate {}, actual {}",
                DEPTH_COMPLEXITY,
                self.hits.len() + 1
            );
        }

        for nhit in core::mem::take(&mut self.hits) {
            hit.collapse(nhit);
        }
        self.hits.push(hit);
    }

    #[inline]
    /// Get hits from front of every lane
    pub fn get(&self) -> Option<[RayHit<'_, Sphere>; 4]> {
        debug_assert!(self.hits.len() <= 1);

        let Some(hit) = self.hits.first() else {
            return None;
        };
        let pos: [Vec3; 4] = (self.pos + hit.t * self.dir).into();
        let dir: [Vec3; 4] = self.dir.into();
        Some(core::array::from_fn(|i| RayHit {
            pos: pos[i],
            dir: dir[i],
            obj: hit.obj[i],
            t: hit.t.as_array_ref()[i],
        }))
    }
}

// TODO make this not generic
struct Hitsx4<'a, T: Primitive> {
    t: f32x4,
    obj: [&'a T; 4],
    uv: (f32x4, f32x4),
}

impl<'a, T: Primitive> Hitsx4<'a, T> {
    fn collapse(&mut self, mut other: Hitsx4<'a, T>) {
        let mask = self.t.is_nan() | (other.t.cmp_lt(self.t) & !other.t.is_nan());
        self.t = mask.blend(other.t, self.t);
        self.uv.0 = mask.blend(other.uv.0, self.uv.0);
        self.uv.1 = mask.blend(other.uv.1, self.uv.1);
        for (i, &mask) in mask.as_array_ref().iter().enumerate() {
            if mask != 0. {
                core::mem::swap(&mut self.obj[i], &mut other.obj[i]);
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct RayHit<'a, T: Primitive> {
    pub pos: Vec3,
    pub dir: Vec3,
    pub obj: &'a T,
    pub t: f32,
}

impl<T: Primitive> RayHit<'_, T> {
    pub fn shade(&self, depth: u16) -> (Vec3, crate::api::RtMaterial) {
        self.obj.shade(self.pos, self.dir, self.t, depth)
    }
}

pub(super) struct ShadowRay {
    pub pos: Vec3,
    pub dir: Vec3,
    pub max_t: f32,
    pub irradiance: Option<Vec3>,
}

impl ShadowRay {
    #[inline]
    pub fn intersect<T: Primitive>(&mut self, primitive: &T, depth: u16) -> bool {
        if primitive.shadow(self, self.max_t, depth) {
            self.irradiance = None;
        }
        self.irradiance.is_none()
    }
}
