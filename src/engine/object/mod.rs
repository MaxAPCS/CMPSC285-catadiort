use ultraviolet::{Vec2, Vec3};

mod lightdirectional;
mod lightmesh;
mod lightplane;
mod lightpoint;
mod lightsphere;
mod lightspot;
mod primitivemesh;
mod primitivesphere;

pub(crate) use lightdirectional::*;
pub(crate) use lightmesh::*;
pub(crate) use lightplane::*;
pub(crate) use lightpoint::*;
pub(crate) use lightsphere::*;
pub(crate) use lightspot::*;
pub(crate) use primitivemesh::*;
pub(crate) use primitivesphere::*;

pub(super) trait Primitive {
    fn bounds(&self) -> (Vec3, Vec3);
    fn centroid(&self) -> Vec3;

    /// Intersect `ray`. t (0, max_t)
    fn obstruct(&self, ray: &mut super::ray::ShadowRay, depth: u16);

    /// Intersect and modify `ray`. t [0, inf)
    fn intersect(&self, ray: &mut super::ray::Rayx4, depth: u16);
}

pub(super) trait Light {
    fn rayto(&self, pos: Vec3, uv: Vec2) -> super::ray::ShadowRay;
    fn cos_angular_radius(&self, p: Vec3) -> f32 {
        let _ = p;
        1.
    }
}

pub(in crate::engine) struct MeshEmissive(pub MeshLight, pub MeshStandin);
pub(in crate::engine) enum MeshStandin {
    Plane(PlaneLight),
    Point(PointLight),
}
impl Light for MeshStandin {
    fn rayto(&self, pos: Vec3, uv: Vec2) -> super::ray::ShadowRay {
        match self {
            Self::Plane(inner) => inner.rayto(pos, uv),
            Self::Point(inner) => inner.rayto(pos, uv),
        }
    }
}

// whittedism
pub(in crate::engine) fn from_emissive(
    triangles: &Box<[Tri]>,
    owtrans: ultraviolet::Mat4,
    material: &crate::api::RtMaterial,
    meshlight: Option<MeshLight>,
) -> Option<MeshEmissive> {
    if material.emission == Vec3::zero() {
        debug_assert!(meshlight.is_none());
        return None;
    }
    let meshlight = meshlight.expect("from_emissive called with None meshlight");

    let owtrans_dual = owtrans.transposed().inversed();
    let wcen = owtrans.transform_point3(Mesh::tricentroid(triangles));
    let wnorm0 = owtrans_dual.transform_vec3(triangles[0].onorm).normalized();
    let mut flat = true;
    let mut doublearea = 0.;
    for tri in triangles {
        let wperp = owtrans_dual.transform_vec3(tri.operp());
        let da = wperp.mag();
        if da == 0.0 {
            continue;
        }
        doublearea += da;
        if wnorm0.dot(wperp / da) <= 0.9998 {
            flat = false;
        }
    }
    let wdoublearea = owtrans.determinant().abs() * doublearea;

    if flat {
        Some(MeshEmissive(
            meshlight,
            MeshStandin::Plane(PlaneLight {
                pos: wcen,
                intensity: material.emission * wdoublearea / 2.,
                norm: wnorm0,
            }),
        ))
    } else {
        Some(MeshEmissive(
            meshlight,
            MeshStandin::Point(PointLight::new(wcen, material.emission * wdoublearea / 8.)),
        ))
    }
}
