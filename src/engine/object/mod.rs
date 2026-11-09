use ultraviolet::{Vec3, Vec3x4};
use wide::f32x4;

use super::ray;

mod lightdirectional;
mod lightplane;
mod lightpoint;
mod lightspot;
mod primitivemesh;
mod primitivesphere;

pub(crate) use lightdirectional::*;
pub(crate) use lightplane::*;
pub(crate) use lightpoint::*;
pub(crate) use lightspot::*;
pub(crate) use primitivemesh::*;
pub(crate) use primitivesphere::*;

pub(super) trait Primitive {
    fn bounds(&self) -> (Vec3, Vec3);
    fn centroid(&self) -> Vec3;

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

pub enum MeshEmissive {
    Plane(PlaneLight),
    Point(PointLight),
}

pub fn from_emissive(o: &Mesh, material: &crate::api::RtMaterial) -> Option<MeshEmissive> {
    if material.emission == Vec3::zero() {
        return None;
    }

    let wnorm0 = o.triangles[0].wnorm();
    let mut flat = true;
    let mut doublearea = 0.;
    for tri in &o.triangles {
        let wperp = tri.wperp();
        let da = wperp.mag();
        doublearea += da;
        if wnorm0.dot(wperp / da) <= 0.9998 {
            flat = false;
        }
    }

    if flat {
        Some(MeshEmissive::Plane(PlaneLight {
            pos: o.centroid(),
            intensity: material.emission * doublearea / 2.,
            norm: wnorm0,
        }))
    } else {
        Some(MeshEmissive::Point(PointLight::new(
            o.centroid(),
            material.emission * doublearea / 8.,
        )))
    }
}
