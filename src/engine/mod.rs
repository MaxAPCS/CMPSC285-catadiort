use std::sync::Arc;
use ultraviolet::Vec3;

use crate::api::{RtMaterial, RtShadeContext, RtShaderFn};

mod object;
mod ray;
mod screen;
mod wider;
mod world;

pub(crate) use object::*;
pub(crate) use screen::*;
pub(crate) use world::*;

const RECAST_EPSILON: f32 = 10e-4;
const RECAST_EPSILON_X4: wide::f32x4 = wide::f32x4::new([RECAST_EPSILON; _]);
const DEPTH_COMPLEXITY: usize = 4;

#[derive(Clone)]
pub(crate) struct Material {
    /// Surface Shader Reference
    pub(super) shader: RtShaderFn,
    pub(super) shaderparams: Option<Arc<[u8]>>,

    /// kd: diffuse reflectance per channel, 0 to 1
    pub(super) albedo: Vec3,

    /// ks: the reflectance head on: it weights the mirror ray and the highlight (HW2), the glossy lobe (HW3), and is Schlick's F0, metal or not (HW4)
    pub(super) specular: Vec3,

    /* radiance emitted (seen directly, this bright). Nonzero makes the draw call a
    light: a point light at its center in HW2, its surface sampled in HW3, with
    MIS in HW4 */
    pub(super) emission: Vec3,

    /* kt: 0 = opaque, 1 = clear glass */
    pub(super) transmission: f32,

    /* 0 = mirror, 1 = fully rough; the lobe's width is alpha = roughness^2 in every
    homework. HW2: the Phong exponent n = 2 / alpha^2 - 2 (alpha clamped to
    [0.05, 1]) and how much of the mirror ray survives; HW3: the glossy lobe's
    width; HW4: GGX's alpha */
    pub(super) roughness: f32,

    /* 0 = dielectric, 1 = metal. HW4 only; HW2 and HW3 ignore it */
    pub(super) metallic: f32,

    /* eta: index of refraction of the inside (1.5 = glass) */
    pub(super) ior: f32,
}

impl Material {
    #[inline]
    const fn reflects(&self) -> bool {
        const SQRT_0D15: f32 = 0.3872983346207416885179265399782399610832921705291590826587;

        self.transmission == 0.
            && self.roughness < SQRT_0D15
            && (self.specular.x > 0. || self.specular.y > 0. || self.specular.z > 0.)
    }

    #[inline]
    const fn refracts(&self) -> bool {
        self.transmission > 0.
    }

    #[inline]
    const fn smoothness(&self) -> f32 {
        (1. - self.roughness * self.roughness * 0.15f32.recip()).max(0.)
    }

    #[inline]
    fn fresnelx4(&self, air_cos: wide::f32x4) -> wide::f32x4 {
        if self.ior == world::MEDIUM_IOR {
            return wide::f32x4::ZERO;
        }
        let f0 = (self.ior - world::MEDIUM_IOR) / (self.ior + world::MEDIUM_IOR);
        let f0 = f0 * f0;
        let t = 1. - air_cos;
        let t2 = t * t;
        f0 + (1. - f0) * (t2 * t2 * t)
    }

    fn shade(
        &self,
        wpos: Vec3,
        wdir: Vec3,
        wnorm: Vec3,
        t: f32,
        opos: Vec3,

        norm: Vec3,
        uv: (f32, f32),
        color: Option<Vec3>,
        depth: u16,
    ) -> (Vec3, RtMaterial) {
        let Some(shader) = self.shader else {
            return (wnorm, self.into());
        };
        let mut material: RtMaterial = self.into();
        if let Some(color) = color {
            material.albedo = color;
        }
        let mut ctx = RtShadeContext::new(
            &self.shaderparams,
            wpos,
            wdir,
            wnorm,
            t,
            opos,
            norm,
            uv,
            depth,
            material,
        );
        unsafe { shader(&mut ctx) };
        (ctx.N.into(), ctx.m)
    }

    pub(crate) const fn from(
        other: RtMaterial,
        shader: RtShaderFn,
        shaderparams: Option<Arc<[u8]>>,
    ) -> Self {
        Self {
            albedo: other.albedo,
            specular: other.specular,
            roughness: other.roughness,
            metallic: other.metallic,
            transmission: other.transmission,
            ior: other.ior,
            emission: other.emission,
            shader,
            shaderparams,
        }
    }

    const fn default() -> Self {
        Self::from(RtMaterial::default_material(), None, None)
    }
}

impl RtMaterial {
    #[inline]
    const fn phongexponent(&self) -> u16 {
        let alpha = (self.roughness * self.roughness).clamp(0.05, 1.);
        (2. / (alpha * alpha) - 2.).max(1.) as _
    }
    /// Arguments:
    /// - obj_wnorm: Object's Worldspace Normal, normalized
    /// - ray_wdir: Ray's Worldspace Direction, normalized
    fn color(
        &self,
        shadowrays: &[ray::ShadowRay],
        obj_norm: Vec3,
        ray_wdir: Vec3,
        ambient: Vec3,
    ) -> Vec3 {
        let mut diffuse = Vec3::zero();
        let mut specular = Vec3::zero();
        for shadowray in shadowrays {
            diffuse += shadowray.irradiance * obj_norm.dot(shadowray.dir).max(0.);
            specular += shadowray.irradiance
                * shadowray
                    .dir
                    .reflected(obj_norm)
                    .dot(ray_wdir)
                    .max(0.)
                    .powi(self.phongexponent() as _)
        }
        (1. - self.transmission) * self.albedo * (ambient + diffuse / core::f32::consts::PI)
            + self.specular * specular
            + self.emission
    }
}

impl Into<RtMaterial> for &Material {
    fn into(self) -> RtMaterial {
        RtMaterial {
            albedo: self.albedo,
            specular: self.specular,
            roughness: self.roughness,
            metallic: self.metallic,
            transmission: self.transmission,
            ior: self.ior,
            emission: self.emission,
        }
    }
}

impl RtShadeContext {
    fn new(
        params: &Option<Arc<[u8]>>,
        wpos: Vec3,
        wdir: Vec3,
        wnorm: Vec3,
        t: f32,
        opos: Vec3,
        norm: Vec3,
        uv: (f32, f32),
        depth: u16,
        material: RtMaterial,
    ) -> Self {
        Self {
            P: wpos,
            Po: opos,
            Ng: wnorm,
            N: norm,
            dPdu: Vec3::zero(),
            dPdv: Vec3::zero(),
            u: uv.0,
            v: uv.1,
            I: wdir,
            time: t,
            depth: depth as _,
            params: params
                .as_ref()
                .map_or(core::ptr::null(), |p| p.as_ptr().cast()),
            m: material,
        }
    }
}
