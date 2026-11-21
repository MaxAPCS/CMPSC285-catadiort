use ultraviolet::{Vec2, Vec3};

use super::{Light, Primitive, primitivesphere::Sphere};
use crate::engine::{PointLight, ray};
use crate::utils::warp;

pub struct SphereLight(Sphere, PointLight);

impl Light for SphereLight {
    fn rayto(&self, wpos: Vec3, uv: Vec2) -> ray::ShadowRay {
        if uv.x.is_nan() {
            return self.1.rayto(wpos, uv);
        }

        let wcen = self.0.centroid();
        let wdir = wcen - wpos;
        let wdstsq = wdir.mag_sq();
        if wdstsq == 0. {
            return ray::ShadowRay::empty();
        }

        let wrad = self.0.extract_avg_scale();
        let wdst = wdstsq.sqrt();
        let wdir = wdir / wdst;

        let ratio = wrad * wrad / wdstsq;
        if wrad * wrad < wdstsq {
            let one_minus_cos_theta_max = ratio / (1. + (1. - ratio).sqrt());
            let wsdir =
                warp::recenter(warp::sample_uniform_cone(uv, one_minus_cos_theta_max), wdir)
                    .normalized();

            let mut ray = ray::ShadowRay::new(wpos, wsdir, f32::INFINITY, Vec3::one());
            self.0.obstruct(&mut ray, u16::MAX);
            if ray.hits.is_empty() {
                return ray::ShadowRay::empty();
            };
            let hit = ray.hits.swap_remove(0);

            let (_, matr) = hit.shade(0); // todo use source depth + 1

            let density_inv = warp::uniform_cone_pdf(one_minus_cos_theta_max).recip();
            ray::ShadowRay::new(wpos, wsdir, hit.t, matr.emission * density_inv)
        } else {
            let rho = wrad * uv.x.sqrt();
            let (sphi, cphi) = (2. * core::f32::consts::PI * uv.y).sin_cos();
            let (t, b) = warp::orthonormal_basis(-wdir);
            let wsend = wcen + t * (rho * cphi) + b * (rho * sphi);

            let wsdir = wsend - wpos;
            let wsdstsq = wsdir.mag_sq();
            let wsdst = wsdstsq.sqrt();
            let wsdir = wsdir / wsdst;

            let scale = core::f32::consts::PI * (wdst / wsdst) * (wdstsq + wrad * wrad) / wsdstsq;
            ray::ShadowRay::new(wpos, wsdir, wsdst, self.0.material.prop.emission * scale)
        }
    }

    fn cos_angular_radius(&self, p: Vec3) -> f32 {
        let dstsq = (p - self.0.centroid()).mag_sq();
        let rad = self.0.extract_avg_scale();
        let radsq = rad * rad;
        if dstsq <= radsq {
            -1.
        } else {
            (1. - radsq / dstsq).sqrt()
        }
    }
}
impl SphereLight {
    pub fn from_emissive(sphere: &Sphere) -> Option<Self> {
        if !sphere.material.prop.emits() {
            return None;
        }

        let scale = sphere.extract_avg_scale();
        let intensity = scale * scale * core::f32::consts::PI * sphere.material.prop.emission;

        Some(Self(
            sphere.clone(),
            PointLight {
                pos: sphere.centroid(),
                intensity,
                radius: scale,
            },
        ))
    }
}
