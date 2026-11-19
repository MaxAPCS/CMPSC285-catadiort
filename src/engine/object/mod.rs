use ultraviolet::Vec3;

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

    /// Intersect `ray`. t (0, max_t)
    fn obstruct(&self, ray: &mut super::ray::ShadowRay, depth: u16);

    /// Intersect and modify `ray`. t [0, inf)
    fn intersect(&self, ray: &mut super::ray::Rayx4, depth: u16);
}

pub(super) trait Light {
    fn rayto(&self, pos: Vec3) -> super::ray::ShadowRay;
}

pub enum MeshEmissive {
    Plane(PlaneLight),
    Point(PointLight),
}

pub fn from_emissive(
    triangles: &Box<[Tri]>,
    owtrans: ultraviolet::Mat4,
    material: &crate::api::RtMaterial,
) -> Option<MeshEmissive> {
    if material.emission == Vec3::zero() {
        return None;
    }

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
        Some(MeshEmissive::Plane(PlaneLight {
            pos: wcen,
            intensity: material.emission * wdoublearea / 2.,
            norm: wnorm0,
        }))
    } else {
        Some(MeshEmissive::Point(PointLight::new(
            wcen,
            material.emission * wdoublearea / 8.,
        )))
    }
}
