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
    const fn specs(&self) -> bool {
        self.specular.x > 0. || self.specular.y > 0. || self.specular.z > 0.
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
