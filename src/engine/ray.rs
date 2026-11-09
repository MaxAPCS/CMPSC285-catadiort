use ultraviolet::{Mat4, Mat4x4, Vec3, Vec3x4, Vec4, Vec4x4};
use wide::{CmpGt, CmpLt, f32x4, u32x4};

use super::DEPTH_COMPLEXITY;
use super::Material;
use super::object::Primitive;
use crate::api::RtMaterial;
use crate::api::RtShadeContext;

pub(super) struct Rayx4 {
    /// Worldspace Ray Origin
    pub pos: Vec3x4,
    /// Worldspace Ray Direction
    pub dir: Vec3x4,

    hits: smallvec::SmallVec<[Hitx4; DEPTH_COMPLEXITY]>,
}

impl Rayx4 {
    #[inline]
    pub fn new(pos: Vec3x4, dir: Vec3x4) -> Self {
        Self {
            pos,
            dir,
            hits: smallvec::SmallVec::new(),
        }
    }

    #[inline]
    /// Insert hit on one object to every lane
    pub fn intersect<T: Primitive>(&mut self, primitive: &T, depth: u16) {
        if let Some(hit) = primitive.intersect(self, depth) {
            self.hits.push(hit);
        };
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

pub(super) struct Hitx4 {
    /// World to Object Transform
    pub wotrans: Mat4x4,
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
    pub material: u32x4,
}

impl Hitx4 {
    // pub(super) to give access to primitivemesh
    pub(super) fn collapse(&mut self, other: Self) {
        let me = (!self.t.is_nan()) & (other.t.is_nan() | self.t.cmp_lt(other.t));

        for i in 0..4 {
            self.wotrans.cols[i] = Vec4x4::blend(me, self.wotrans.cols[i], other.wotrans.cols[i]);
        }
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
        self.material = u32x4::blend(bytemuck::cast(me), self.material, other.material);
    }

    pub fn shade(&self, material: &Material, depth: u16) -> (Vec3x4, [RtMaterial; 4]) {
        let hits: [Hit; 4] = self.into();
        let opos: [Vec3; 4] = self.wotrans.transform_point3(self.wpos).into();

        let mut out_norms = [Vec3::zero(); 4];
        let mut out_matrs = [const { core::mem::MaybeUninit::uninit() }; 4];
        for i in 0..4 {
            let (norm, matr) = hits[i].shade(opos[i], material, depth);
            out_norms[i] = norm;
            out_matrs[i].write(matr);
        }
        (out_norms.into(), unsafe { core::mem::transmute(out_matrs) })
    }

    #[inline]
    pub fn reflect(&self) -> Vec3x4 {
        self.wdir.reflected(self.norm)
    }

    #[inline]
    pub fn refract(&self, material: &Material) -> (f32x4, Vec3x4) {
        let entering = self.wdir.dot(self.wnorm).cmp_lt(f32x4::ZERO);
        let norm = Vec3x4::blend(
            self.wdir.dot(self.norm).cmp_gt(f32x4::ZERO),
            -self.norm,
            self.norm,
        );
        let wscale = self.wdir.mag();
        let (medium_cos, refractdir) =
            refractedx4(self.wdir / wscale, norm, material.ior.into(), entering);
        (medium_cos, refractdir * wscale)
    }
}

/// Arguments:
/// - dir: normalized, either direction
/// - norm: normalized, away from surface
/// - eta: object ior / medium ior
/// Returns:
/// - medium_cos: cosine between dir to norm on medium side
/// - refractdir: NaN if TIR, normalized
#[inline]
fn refractedx4(dir: Vec3x4, norm: Vec3x4, eta: f32x4, entering: f32x4) -> (f32x4, Vec3x4) {
    let ndi = norm.dot(dir);
    let eta = entering.blend(eta.recip(), eta);
    let k = 1.0 - eta * eta * (1.0 - ndi * ndi);
    let sqrtk = k.sqrt(); // medium side cos, or NaN on TIR
    (
        k.cmp_lt(f32x4::ZERO).blend(f32x4::ZERO, -ndi),
        dir.mul_add(Vec3x4::broadcast(eta), -(eta * ndi + sqrtk) * norm),
    )
}

#[derive(bytemuck::Zeroable)]
pub(super) struct Hit {
    /// World to Object Transform
    pub wotrans: Mat4,
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
    pub material: u32,
}

impl Hit {
    fn shade(&self, opos: Vec3, material: &Material, depth: u16) -> (Vec3, RtMaterial) {
        let mut matr: RtMaterial = material.into();
        if !self.color.x.is_nan() {
            matr.albedo = self.color;
        };
        let Some(shader) = material.shader else {
            return (self.norm, matr);
        };
        let mut ctx = RtShadeContext::new(
            &material.shaderparams,
            self.wpos,
            self.wdir,
            self.wnorm,
            self.t,
            opos,
            self.norm,
            self.uv,
            depth,
            matr,
        );
        unsafe { shader(&mut ctx) };
        (ctx.N, ctx.m)
    }
}

impl Into<[Hit; 4]> for &Hitx4 {
    fn into(self) -> [Hit; 4] {
        let wotrans: [[Vec4; 4]; 4] = self.wotrans.cols.map(|c| c.into());
        let wpos: [Vec3; 4] = self.wpos.into();
        let wdir: [Vec3; 4] = self.wdir.into();
        let wnorm: [Vec3; 4] = self.wnorm.into();
        let t = self.t.as_array_ref();
        let norm: [Vec3; 4] = self.norm.into();
        let u = self.uv.0.as_array_ref();
        let v = self.uv.1.as_array_ref();
        let color: [Vec3; 4] = self.color.into();
        let material = self.material.as_array_ref();

        core::array::from_fn(|i| Hit {
            wotrans: Mat4::new(wotrans[0][i], wotrans[1][i], wotrans[2][i], wotrans[3][i]),
            wpos: wpos[i],
            wdir: wdir[i],
            wnorm: wnorm[i],
            t: t[i],
            norm: norm[i],
            uv: (u[i], v[i]),
            color: color[i],
            material: material[i],
        })
    }
}

impl From<[Hit; 4]> for Hitx4 {
    fn from(value: [Hit; 4]) -> Self {
        let wotrans: [Vec4x4; 4] = core::array::from_fn(|i| value[i].wotrans.cols.into());
        let wpos: [Vec3; 4] = core::array::from_fn(|i| value[i].wpos);
        let wdir: [Vec3; 4] = core::array::from_fn(|i| value[i].wdir);
        let wnorm: [Vec3; 4] = core::array::from_fn(|i| value[i].wnorm);
        let t: [f32; 4] = core::array::from_fn(|i| value[i].t);
        let norm: [Vec3; 4] = core::array::from_fn(|i| value[i].norm);
        let u: [f32; 4] = core::array::from_fn(|i| value[i].uv.0);
        let v: [f32; 4] = core::array::from_fn(|i| value[i].uv.1);
        let color: [Vec3; 4] = core::array::from_fn(|i| value[i].color);
        let material: [u32; 4] = core::array::from_fn(|i| value[i].material);

        Self {
            wotrans: Mat4x4 { cols: wotrans },
            wpos: wpos.into(),
            wdir: wdir.into(),
            wnorm: wnorm.into(),
            t: t.into(),
            norm: norm.into(),
            uv: (u.into(), v.into()),
            color: color.into(),
            material: material.into(),
        }
    }
}

pub(super) struct ShadowRay {
    /// Worldspace Ray Origin
    pub pos: Vec3,
    /// Worldspace Ray Direction
    pub dir: Vec3,
    pub max_t: f32,
    pub irradiance: Vec3,
}

impl ShadowRay {
    #[inline]
    pub fn intersect<T: Primitive>(&mut self, primitive: &T, depth: u16) -> bool {
        if primitive.obstruct(self, depth) {
            self.irradiance = Vec3::zero();
        }
        self.irradiance == Vec3::zero()
    }
}
