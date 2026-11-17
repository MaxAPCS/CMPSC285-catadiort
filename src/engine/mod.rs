use ultraviolet::Vec3;

use crate::api::{RtMaterial, RtShadeContext, RtShaderFn};

#[cfg(feature = "bvh")]
mod bvh;
mod object;
mod ray;
mod screen;
mod wider;
mod world;

pub use object::*;
pub(crate) use screen::*;
pub(crate) use world::*;

#[derive(Clone)]
pub(crate) struct Material {
    pub(super) id: u16,

    /// Surface Shader Reference
    pub(super) shader: RtShaderFn,
    pub(super) shaderparams: Option<std::sync::Arc<[u8]>>,

    pub(super) prop: RtMaterial,
}

impl Material {
    #[inline]
    const fn reflects(&self) -> bool {
        const SQRT_0D15: f32 = 0.38729835;

        self.prop.transmission == 0.
            && self.prop.roughness < SQRT_0D15
            && (self.prop.specular.x > 0. || self.prop.specular.y > 0. || self.prop.specular.z > 0.)
    }

    #[inline]
    const fn refracts(&self) -> bool {
        self.prop.transmission > 0.
    }

    #[inline]
    const fn smoothness(&self) -> f32 {
        const RECIP_0D15: f32 = 0.15f32.recip();

        (1. - self.prop.roughness * self.prop.roughness * RECIP_0D15).max(0.)
    }

    /// Arguments:
    /// - medium_cos: cosine between incident and medium-facing normal
    /// - object_ior: index of refraction of object
    #[inline]
    fn fresnelx4(medium_cos: wide::f32x4, object_ior: wide::f32x4) -> wide::f32x4 {
        use wide::{CmpEq, f32x4};
        let sameior = object_ior.cmp_eq(world::MEDIUM_IOR);
        if sameior.all() {
            return f32x4::ZERO;
        }
        let f0 = (object_ior - 1.) / (object_ior + 1.);
        let f0 = f0 * f0;
        let t = f32x4::ONE - medium_cos;
        let t2 = t * t;
        sameior.blend(f32x4::ZERO, f0 + (f32x4::ONE - f0) * t2 * t2 * t)
    }

    pub(crate) const fn new(
        id: u16,
        shader: RtShaderFn,
        shaderparams: Option<std::sync::Arc<[u8]>>,
        prop: RtMaterial,
    ) -> Self {
        Self {
            id,
            shader,
            shaderparams,
            prop,
        }
    }

    pub(crate) const fn default() -> Self {
        Self::new(0, None, None, RtMaterial::default_material())
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
