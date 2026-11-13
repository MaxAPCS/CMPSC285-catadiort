use ultraviolet::{Mat4, Vec3};

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
    #[allow(dead_code)]
    fn bounds(&self) -> (Vec3, Vec3);
    fn centroid(&self) -> Vec3;

    fn obstruct(&self, ray: &ray::ShadowRay, depth: u16) -> bool;
    fn intersect(&self, rays: &mut ray::Rayx4, depth: u16);
}

pub(super) trait Light {
    fn rayto(&self, pos: Vec3) -> ray::ShadowRay;
}

pub enum MeshEmissive {
    Plane(PlaneLight),
    Point(PointLight),
}

pub fn from_emissive(
    triangles: &Box<[Tri]>,
    owtrans_dual: Mat4,
    material: &crate::api::RtMaterial,
) -> Option<MeshEmissive> {
    if material.emission == Vec3::zero() {
        return None;
    }

    let wnorm0 = owtrans_dual.transform_vec3(triangles[0].onorm());
    let mut flat = true;
    let mut doublearea = 0.;
    for tri in triangles {
        let wperp = owtrans_dual.transform_vec3(tri.operp());
        let da = wperp.mag();
        doublearea += da;
        if wnorm0.dot(wperp / da) <= 0.9998 {
            flat = false;
        }
    }

    if flat {
        Some(MeshEmissive::Plane(PlaneLight {
            pos: Mesh::tricentroid(triangles),
            intensity: material.emission * doublearea / 2.,
            norm: wnorm0,
        }))
    } else {
        Some(MeshEmissive::Point(PointLight::new(
            Mesh::tricentroid(triangles),
            material.emission * doublearea / 8.,
        )))
    }
}
