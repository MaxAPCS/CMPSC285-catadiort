use ultraviolet::{Vec3, Vec3x4};
use wide::{f32x4, f64x4};

use super::ray::{Rayx4, ShadowRay};
use crate::api::{RtMaterial, RtShadeContext};

mod lightdirectional;
mod lightpoint;
mod lightspot;
mod primitivemesh;
mod primitivesphere;

pub(crate) use lightdirectional::*;
pub(crate) use lightpoint::*;
pub(crate) use lightspot::*;
pub(crate) use primitivesphere::*;

pub(super) trait Primitive {
    fn bounds(&self) -> (Vec3, Vec3);

    /// Return: wdir out
    fn reflect(&self, wpos: Vec3, wdir: Vec3) -> Vec3;
    fn reflectx4(&self, wpos: Vec3x4, wdir: Vec3x4) -> Vec3x4;
    fn reflects(&self) -> bool;

    /// Return: cosine, wdir out (NAN if total internal reflection)
    fn refract(&self, wpos: Vec3, wdir: Vec3) -> (f32, Vec3);
    fn refractx4(&self, wpos: Vec3x4, wdir: Vec3x4) -> (f32x4, Vec3x4);
    fn refracts(&self) -> bool;

    fn shade(&self, wpos: Vec3, wdir: Vec3, t: f32, depth: u16) -> (Vec3, RtMaterial);
    fn shadex4(
        &self,
        wpos: Vec3x4,
        wdir: Vec3x4,
        t: f32x4,
        depth: u16,
    ) -> (Vec3x4, [RtMaterial; 4]);

    /// Return: true if surface exists
    fn cutout(&self, wpos: Vec3, wdir: Vec3, opos: Vec3, t: f32, depth: u16) -> bool;
    fn cutoutx4(&self, wpos: Vec3x4, wdir: Vec3x4, opos: Vec3x4, t: f32x4, depth: u16) -> f32x4;

    fn shadow(&self, ray: &ShadowRay, tmax: f32, depth: u16) -> bool;
    /// Return: t when the vector first meets this shape
    fn intersectx4(&self, rays: &Rayx4, depth: u16) -> f32x4;
}

pub(super) trait Light {
    fn rayto(&self, pos: Vec3) -> ShadowRay;
}

fn shadecontext(
    wpos: Vec3,
    wdir: Vec3,
    wnorm: Vec3,
    opos: Vec3,
    time: f32,
    uv: (f32, f32),
    depth: u16,
    params: *const std::ffi::c_void,
    material: RtMaterial,
) -> RtShadeContext {
    RtShadeContext {
        P: *wpos.as_array(),
        Po: *opos.as_array(),
        Ng: *wnorm.as_array(),
        N: *wnorm.as_array(),
        dPdu: [0.; _],
        dPdv: [0.; _],
        u: uv.0,
        v: uv.1,
        I: *wdir.as_array(),
        time,
        depth: depth as _,
        params,
        m: material,
    }
}
