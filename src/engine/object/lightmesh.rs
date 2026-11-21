use std::sync::Arc;
use ultraviolet::{Mat4, Vec2, Vec3};

use super::{Light, Primitive, primitivemesh::*};
use crate::api::{RtCutoutFn, RtShadeContext};
use crate::engine::{Material, ray};
use crate::utils::{hashing::ONE_MINUS_EPSILON, warp};

pub struct MeshLight {
    /// World to Object Transform
    wotrans: Mat4,
    /// Dual of Object to World Transform
    owtrans_dual: Mat4,

    /// Cutout Shader Reference
    cutout: RtCutoutFn,
    cutoutparams: Option<Arc<[u8]>>,

    /// Material Properties
    material: Arc<Material>,

    tris: Arc<[(f32, Tri)]>,

    centroid: Vec3,
    extent: f32,
}

impl Light for MeshLight {
    fn rayto(&self, wpos: Vec3, uv: Vec2) -> ray::ShadowRay {
        // Total Area
        let ta = self.tris.last().unwrap().0;
        // Sample Indexing Area
        let ua = uv.x * ta;
        // Sampled Triangle Index
        let idx = self
            .tris
            .partition_point(|&(a, _)| a < ua)
            .min(self.tris.len() - 1);
        let tri = &self.tris[idx];

        // Previous Triangle Cumulative Area
        let pa = if idx == 0 { 0. } else { self.tris[idx - 1].0 };
        // Sampled Triangle Barycentric Coordinates
        let tribc = warp::sample_uniform_triangle(Vec2::new(
            ((ua - pa) / (tri.0 - pa)).min(ONE_MINUS_EPSILON),
            uv.y,
        ));
        // Sampled Triangle Position
        let tripos = tri.1.pos(tribc);

        let opos = self.wotrans.transform_point3(wpos);
        let odir = tripos - opos;
        let odst2 = odir.mag_sq();
        let odst = odst2.sqrt();
        let odir = odir / odst;

        let incidence = tri.1.onorm.dot(-odir);
        if incidence <= 0. {
            return ray::ShadowRay::empty();
        }

        let mut hit = tri.1.hit(opos, odir, odst, (tribc.y, tribc.z));
        hit.opos = hit.wpos;
        hit.wpos = wpos;
        hit.wdir = self.wotrans.inversed().transform_vec3(hit.wdir);
        hit.wnorm = self.owtrans_dual.transform_vec3(hit.wnorm).normalized();
        hit.norm = self.owtrans_dual.transform_vec3(hit.norm).normalized();
        hit.material = Some(self.material.clone());
        if !self.cutout(&hit, u16::MAX) {
            return ray::ShadowRay::empty();
        }
        let (_, matr) = hit.shade(1); // todo use source depth + 1

        let density_inv = (ta * incidence) / odst2;
        ray::ShadowRay::new(hit.wpos, hit.wdir, odst, matr.emission * density_inv)
    }

    fn cos_angular_radius(&self, p: Vec3) -> f32 {
        let p = self.wotrans.inversed().transform_point3(p);
        let dstsq = (p - self.centroid).mag_sq();
        let rad = self.extent * self.extract_avg_scale();
        let radsq = rad * rad;
        if dstsq < radsq {
            -1.
        } else {
            (1. - radsq / dstsq).sqrt()
        }
    }
}
impl MeshLight {
    pub fn emissive_create(triangles: &Box<[Tri]>, meshref: &MeshReference) -> Option<Self> {
        if !meshref.material.prop.emits() {
            return None;
        }

        let mut atris = Vec::with_capacity(triangles.len());
        let mut numer = Vec3::zero();
        let mut denom = 0.;
        for tri in triangles {
            let area = tri.operp().mag();
            numer += tri.centroid() * area;
            denom += area;
            atris.push((denom, tri.clone()));
        }
        let centroid = numer / denom;

        let wotrans = meshref.owtrans.inversed();
        Some(Self {
            wotrans,
            owtrans_dual: wotrans.transposed(),
            cutout: meshref.cutout,
            cutoutparams: meshref.cutoutparams.clone(),
            material: meshref.material.clone(),
            tris: atris.into(),
            centroid,
            extent: Mesh::triextent(triangles, centroid),
        })
    }

    pub fn emissive_instance(other: Option<&Self>, meshref: &MeshReference) -> Option<Self> {
        let Some(other) = other else {
            return None;
        };
        if !meshref.material.prop.emits() {
            return None;
        }
        let wotrans = meshref.owtrans.inversed();
        Some(Self {
            wotrans,
            owtrans_dual: wotrans.transposed(),
            cutout: meshref.cutout,
            cutoutparams: meshref.cutoutparams.clone(),
            material: meshref.material.clone(),
            tris: other.tris.clone(),
            centroid: other.centroid,
            extent: other.extent,
        })
    }

    #[inline]
    fn cutout(&self, hit: &ray::Hit, depth: u16) -> bool {
        let Some(cutout) = self.cutout else {
            return true;
        };
        let mut ctx = RtShadeContext::new(
            &self.cutoutparams,
            hit.wpos,
            hit.wdir,
            hit.wnorm,
            hit.t,
            hit.opos,
            hit.norm,
            hit.uv,
            depth,
            self.material.prop,
        );
        (unsafe { cutout(&mut ctx) } != 0)
    }

    #[inline]
    fn extract_avg_scale(&self) -> f32 {
        let inv3 = self.wotrans.inversed().truncate();
        (inv3.cols[0].mag() + inv3.cols[1].mag() + inv3.cols[2].mag()) / 3.
    }
}
