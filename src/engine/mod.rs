use ultraviolet::Vec3;

use crate::api::{RtMaterial, RtShadeContext, RtShaderFn};

#[cfg(feature = "bvh")]
mod bvh;
pub(crate) mod integrators;
mod object;
mod ray;
mod screen;
mod wider;
mod world;

pub(crate) use object::*;
pub(crate) use screen::*;
pub(crate) use world::*;

#[derive(Clone)]
pub(crate) struct Material {
    /// Surface Shader Reference
    pub(super) shader: RtShaderFn,
    pub(super) shaderparams: Option<std::sync::Arc<[u8]>>,

    pub(super) prop: RtMaterial,
}

impl Material {
    pub(crate) const fn new(
        shader: RtShaderFn,
        shaderparams: Option<std::sync::Arc<[u8]>>,
        prop: RtMaterial,
    ) -> Self {
        Self {
            shader,
            shaderparams,
            prop,
        }
    }

    pub(crate) const fn default() -> Self {
        Self::new(None, None, RtMaterial::default_material())
    }
}

impl RtMaterial {
    #[inline]
    pub const fn emits(&self) -> bool {
        self.emission.x > 0. || self.emission.y > 0. || self.emission.z > 0.
    }

    #[inline]
    const fn reflects(&self) -> bool {
        const SQRT_0D15: f32 = 0.38729835;

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
        const RECIP_0D15: f32 = 0.15f32.recip();

        (1. - self.roughness * self.roughness * RECIP_0D15).max(0.)
    }

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
        shadowrays: impl IntoIterator<Item = ray::ShadowRay>,
        obj_norm: Vec3,
        ray_wdir: Vec3,
        ambient: Vec3,
    ) -> Vec3 {
        let phongexp = self.phongexponent() as i32;
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
                    .powi(phongexp)
        }
        (1. - self.transmission) * self.albedo * (ambient + diffuse / core::f32::consts::PI)
            + self.specular * specular
            + self.emission
    }
}

impl RtShadeContext {
    fn new(
        params: &Option<std::sync::Arc<[u8]>>,
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
            dPdu: Vec3::broadcast(0.),
            dPdv: Vec3::broadcast(0.),
            u: uv.0,
            v: uv.1,
            I: wdir,
            time: t,
            depth: if depth == u16::MAX { -1 } else { depth as _ },
            params: params
                .as_ref()
                .map_or(core::ptr::null(), |p| p.as_ptr().cast()),
            m: material,
        }
    }
}
