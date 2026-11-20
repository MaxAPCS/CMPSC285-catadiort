use std::sync::Arc;
use ultraviolet::{Mat4, Vec3, Vec3x4};
use wide::{CmpLe, f32x4, u32x4};

use super::{Material, wider};
use crate::{
    api::{RtMaterial, RtShadeContext},
    utils::samplers::AnySamplerx4,
};

const DEPTH_COMPLEXITY: usize = 2;

pub(super) struct Rayx4 {
    /// Worldspace Ray Origin
    pub pos: Vec3x4,
    /// Worldspace Ray Direction
    pub dir: Vec3x4,
    /// Furthest hit
    pub max_t: f32x4,

    pub sampler: AnySamplerx4,

    pub hits: smallvec::SmallVec<[Hitx4; DEPTH_COMPLEXITY]>, // only for PrimitiveMesh
}

impl Rayx4 {
    #[inline]
    pub const fn new(pos: Vec3x4, dir: Vec3x4, sampler: AnySamplerx4) -> Self {
        Self {
            pos,
            dir,
            max_t: f32x4::new([f32::INFINITY; _]),
            sampler,
            hits: smallvec::SmallVec::new_const(),
        }
    }

    #[inline]
    pub fn transformed(&self, trans: &Mat4) -> Self {
        Self {
            pos: wider::transform_pointx4(trans, &self.pos),
            dir: wider::transform_vecx4(trans, self.dir),
            max_t: self.max_t,
            sampler: self.sampler.clone(),
            hits: smallvec::SmallVec::new_const(),
        }
    }

    #[inline]
    pub fn push(&mut self, hit: Hitx4) {
        self.max_t = self.max_t.min(hit.t);
        for i in (0..self.hits.len()).rev() {
            if self.hits[i].t.cmp_le(self.max_t).none() {
                self.hits.remove(i);
            }
        }
        self.hits.push(hit);
    }

    #[inline]
    /// Collapse hits combining each lane's by distance
    pub fn collapse(&mut self) {
        let Some(mut hit) = self.hits.pop() else {
            return;
        };
        for nhit in core::mem::take(&mut self.hits) {
            hit.collapse(nhit);
        }
        self.hits.push(hit);
    }

    #[inline]
    /// Get hits from front of every lane
    pub fn get(&mut self) -> Option<Hitx4> {
        debug_assert!(self.hits.len() <= 1);
        self.hits.pop()
    }
}

macro_rules! material {
    ($material:expr) => {
        $material
            .as_deref()
            .unwrap_or(const { &Material::default() })
    };
}

pub(super) struct Hitx4 {
    /// Objectspace Hit Position
    pub opos: Vec3x4,
    /// Worldspace Hit Position
    pub wpos: Vec3x4,
    /// Worldspace Incidient Direction
    pub wdir: Vec3x4,
    /// Worldspace Surface Normal, normalized
    pub wnorm: Vec3x4,
    /// Ray Time
    pub t: f32x4,

    /// Shading Normal, normalized, flipped
    pub norm: Vec3x4,
    /// Shading UV
    pub uv: (f32x4, f32x4),
    /// Shading Color
    pub color: Vec3x4,
    /// Shading Material
    pub material: [Option<Arc<Material>>; 4],
}

impl Hitx4 {
    fn collapse(&mut self, mut other: Self) {
        let me = (!self.t.is_nan()) & (other.t.is_nan() | self.t.cmp_le(other.t));

        self.opos = Vec3x4::blend(me, self.opos, other.opos);
        self.wpos = Vec3x4::blend(me, self.wpos, other.wpos);
        self.wdir = Vec3x4::blend(me, self.wdir, other.wdir);
        self.wnorm = Vec3x4::blend(me, self.wnorm, other.wnorm);
        self.t = f32x4::blend(me, self.t, other.t);

        self.norm = Vec3x4::blend(me, self.norm, other.norm);
        self.uv = (
            f32x4::blend(me, self.uv.0, other.uv.0),
            f32x4::blend(me, self.uv.1, other.uv.1),
        );
        self.color = Vec3x4::blend(me, self.color, other.color);

        let me: u32x4 = bytemuck::cast(me);
        let me = me.as_array_ref();
        for i in 0..4 {
            if me[i] == 0 {
                core::mem::swap(&mut self.material[i], &mut other.material[i]);
            }
        }
    }

    #[inline]
    pub fn reflect(&self) -> Vec3x4 {
        self.wdir.reflected(self.norm)
    }
}

#[derive(Default)]
pub(super) struct Hit {
    /// Objectspace Hit Position
    pub opos: Vec3,
    /// Worldspace Hit Position
    pub wpos: Vec3,
    /// Worldspace Incident Direction (towards wpos)
    pub wdir: Vec3,
    /// Worldspace Surface Normal (opposite wdir), normalized
    pub wnorm: Vec3,
    /// Ray Time
    pub t: f32,

    /// Shading Normal (opposite wdir), normalized
    pub norm: Vec3,
    /// Shading UV
    pub uv: (f32, f32),
    /// Shading Color
    pub color: Vec3,
    /// Shading Material
    pub material: Option<Arc<Material>>,
}

impl Hit {
    pub fn shade(&self, depth: u16) -> (Vec3, RtMaterial) {
        let material = material!(self.material);
        let mut matr = material.prop;
        if !self.color.x.is_nan() {
            matr.albedo = self.color;
        };
        if self.t.is_nan() {
            return (self.norm, matr);
        }
        let Some(shader) = material.shader else {
            return (self.norm, matr);
        };
        let mut ctx = RtShadeContext::new(
            &material.shaderparams,
            self.wpos,
            self.wdir,
            self.wnorm,
            self.t,
            self.opos,
            self.norm,
            self.uv,
            depth,
            matr,
        );
        unsafe { shader(&mut ctx) };
        (ctx.N, ctx.m)
    }

    #[inline]
    pub fn reflect(&self) -> Vec3 {
        self.wdir.reflected(self.norm)
    }
}

impl Into<[Hit; 4]> for Hitx4 {
    fn into(self) -> [Hit; 4] {
        let opos: [Vec3; 4] = self.opos.into();
        let wpos: [Vec3; 4] = self.wpos.into();
        let wdir: [Vec3; 4] = self.wdir.into();
        let wnorm: [Vec3; 4] = self.wnorm.into();
        let t = self.t.as_array_ref();
        let norm: [Vec3; 4] = self.norm.into();
        let u = self.uv.0.as_array_ref();
        let v = self.uv.1.as_array_ref();
        let color: [Vec3; 4] = self.color.into();

        let mut i = 0;
        self.material.map(|material| {
            let h = Hit {
                opos: opos[i],
                wpos: wpos[i],
                wdir: wdir[i],
                wnorm: wnorm[i],
                t: t[i],
                norm: norm[i],
                uv: (u[i], v[i]),
                color: color[i],
                material,
            };
            i += 1;
            h
        })
    }
}

impl From<[Hit; 4]> for Hitx4 {
    fn from(value: [Hit; 4]) -> Self {
        let opos = core::array::from_fn(|i| value[i].opos);
        let wpos = core::array::from_fn(|i| value[i].wpos);
        let wdir = core::array::from_fn(|i| value[i].wdir);
        let wnorm = core::array::from_fn(|i| value[i].wnorm);
        let t = core::array::from_fn(|i| value[i].t);
        let norm = core::array::from_fn(|i| value[i].norm);
        let u = core::array::from_fn(|i| value[i].uv.0);
        let v = core::array::from_fn(|i| value[i].uv.1);
        let color = core::array::from_fn(|i| value[i].color);

        Self {
            opos: opos.into(),
            wpos: wpos.into(),
            wdir: wdir.into(),
            wnorm: wnorm.into(),
            t: t.into(),
            norm: norm.into(),
            uv: (u.into(), v.into()),
            color: color.into(),
            material: value.map(|val| val.material),
        }
    }
}

pub(super) struct ShadowRay {
    /// Worldspace Ray Origin
    pub pos: Vec3,
    /// Worldspace Ray Direction
    pub dir: Vec3,
    /// Maximum Hit Time (exclusive)
    pub max_t: f32,

    pub irradiance: Vec3,
    pub hits: smallvec::SmallVec<[Hit; DEPTH_COMPLEXITY]>,
}

impl ShadowRay {
    #[inline]
    pub const fn new(pos: Vec3, dir: Vec3, max_t: f32, irradiance: Vec3) -> Self {
        Self {
            pos,
            dir,
            max_t,
            irradiance,
            hits: smallvec::SmallVec::new_const(),
        }
    }

    #[inline]
    pub fn collapse(&mut self) {
        for hit in core::mem::take(&mut self.hits) {
            let material = material!(hit.material);
            if material.prop.emits() {
                // fixme bandaid
                continue;
            }
            self.irradiance *= material.prop.albedo * material.prop.transmission;
            if self.obstructed() {
                break;
            }
        }
    }

    #[inline]
    pub fn obstructed(&self) -> bool {
        self.irradiance.x <= 0. && self.irradiance.y <= 0. && self.irradiance.z <= 0.
    }
}
