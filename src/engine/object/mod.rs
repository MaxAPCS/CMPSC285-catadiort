use ultraviolet::{Vec3, Vec3x4};
use wide::f32x4;

use super::ray;

mod lightdirectional;
mod lightpoint;
mod lightspot;
mod primitivemesh;
mod primitivesphere;

pub(crate) use lightdirectional::*;
pub(crate) use lightpoint::*;
pub(crate) use lightspot::*;
pub(crate) use primitivemesh::*;
pub(crate) use primitivesphere::*;

pub(super) trait Primitive {
    fn bounds(&self) -> (Vec3, Vec3);

    fn cutout(
        &self,
        wpos: Vec3,
        wdir: Vec3,
        t: f32,
        opos: Vec3,
        uv: (f32, f32),
        depth: u16,
    ) -> bool;
    fn cutoutx4(
        &self,
        wpos: Vec3x4,
        wdir: Vec3x4,
        t: f32x4,
        opos: Vec3x4,
        uv: (f32x4, f32x4),
        depth: u16,
    ) -> f32x4;

    fn obstruct(&self, ray: &ray::ShadowRay, depth: u16) -> bool;
    fn intersect(&self, ray: &ray::Rayx4, depth: u16) -> Option<ray::Hitx4>;
}

pub(super) trait Light {
    fn rayto(&self, pos: Vec3) -> ray::ShadowRay;
}
