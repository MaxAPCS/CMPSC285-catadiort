use ultraviolet::{Vec3, Vec3x4};

use super::{Integrator, RECAST_EPSILON};
use crate::engine::{MEDIUM_IOR, World, ray, wider};
use crate::utils::samplers;

#[derive(Clone, Copy)]
pub struct Whitted;

impl Integrator for Whitted {
    #[inline]
    fn integrate(
        &self,
        world: &World,
        idxs: &[u32; 4],
        wghs: Vec3x4,
        rays: &mut ray::Rayx4,
        depth: u16,
    ) -> (
        ([u32; 4], [Vec3; 4]),
        smallvec::SmallVec<[(u32, Vec3, Vec3, Vec3, samplers::AnySampler); 8]>,
    ) {
        let mut children = smallvec::SmallVec::new();
        let Some(hits) = rays.get() else {
            return (
                (*idxs, (wghs * world.sample_environmentx4(rays.dir)).into()),
                children,
            );
        };
        let mut out = [Vec3::zero(); 4];
        let wghs: [Vec3; 4] = wghs.into();
        let hits: [ray::Hit; 4] = hits.into();
        for (i, mut hit) in hits.into_iter().enumerate() {
            let idx = idxs[i];
            let wgh = wghs[i];

            if wgh.component_max() <= 0. {
                continue;
            }
            if hit.t.is_nan() {
                out[i] = wgh * world.sample_environment(hit.wdir);
                continue;
            }

            // Shade
            hit.norm = wider::copysign(hit.norm, -hit.wdir);
            let (norm, matr) = hit.shade(depth);
            hit.norm = wider::copysign(norm.normalized(), -hit.wdir); // write back shader norms

            // Color
            let alpha = (matr.roughness * matr.roughness).clamp(0.05, 1.);
            let phongexp = (2. / (alpha * alpha) - 2.).max(1.);
            let mut diffuse = Vec3::zero();
            let mut specular = Vec3::zero();
            let shadowrays = world.shadowrays(hit.wpos).filter_map(|mut sr| {
                if sr.obstructed() || hit.norm.dot(sr.dir) <= 0.0 {
                    return None;
                }
                sr.pos += wider::copysign(hit.wnorm * RECAST_EPSILON, sr.dir);
                sr.max_t -= RECAST_EPSILON;
                world.obstruct(&mut sr, u16::MAX);
                if sr.obstructed() { None } else { Some(sr) }
            });
            for shadowray in shadowrays {
                diffuse += shadowray.irradiance * hit.norm.dot(shadowray.dir).max(0.);
                specular += shadowray.irradiance
                    * shadowray
                        .dir
                        .reflected(hit.norm)
                        .dot(hit.wdir)
                        .max(0.)
                        .powf(phongexp)
            }
            out[i] = wgh
                * (1. - matr.transmission)
                * matr.albedo
                * (world.ambientcolor + diffuse * core::f32::consts::FRAC_1_PI)
                + matr.specular * specular
                + matr.emission;

            // Reflect
            const SQRT_0D15: f32 = 0.38729835;
            const RECIP_0D15: f32 = 0.15f32.recip();
            if matr.transmission == 0. && matr.roughness < SQRT_0D15 && matr.specs() {
                let reflectdir = hit.reflect();
                children.push((
                    idx,
                    wgh * matr.specular
                        * (1. - matr.roughness * matr.roughness * RECIP_0D15).max(0.),
                    hit.wpos + wider::copysign(hit.wnorm, reflectdir) * RECAST_EPSILON,
                    reflectdir,
                    samplers::AnySampler::default(),
                ))
            }

            // Refract
            if matr.transmission > 0. {
                let (fresnel, refractdir) =
                    refraction(hit.wdir, hit.norm, matr.ior, hit.wdir.dot(hit.wnorm) <= 0.);
                let reflectdir = hit.reflect();
                children.push((
                    idx,
                    wgh * matr.specular * fresnel,
                    hit.wpos + wider::copysign(hit.wnorm, reflectdir) * RECAST_EPSILON,
                    reflectdir,
                    samplers::AnySampler::default(),
                ));
                if !refractdir.x.is_nan() {
                    children.push((
                        idx,
                        wgh * matr.transmission * (1. - fresnel),
                        hit.wpos + wider::copysign(hit.wnorm, refractdir) * RECAST_EPSILON,
                        refractdir,
                        samplers::AnySampler::default(),
                    ));
                }
            }
        }
        ((*idxs, out), children)
    }
}

/// Arguments:
/// - dir: normalized, towards surface
/// - norm: normalized, away from object center
/// - object_ior: index of refraction of object
/// Returns:
/// - fresnel: reflection ray weight
/// - refractdir: NaN if TIR, normalized
#[inline]
pub(super) fn refraction(dir: Vec3, norm: Vec3, object_ior: f32, entering: bool) -> (f32, Vec3) {
    if object_ior == MEDIUM_IOR {
        return (0., dir);
    }
    let ndi = norm.dot(dir);
    let eta = if entering {
        MEDIUM_IOR / object_ior
    } else {
        object_ior / MEDIUM_IOR
    };
    let k = 1.0 - eta * eta * (1.0 - ndi * ndi);
    let sqrtk = k.sqrt();
    (
        if k < 0. {
            1. // TIR
        } else {
            fresnel(-ndi, object_ior)
        },
        dir.mul_add(Vec3::broadcast(eta), -(eta * ndi + sqrtk) * norm),
    )
}

/// Arguments:
/// - medium_cos: cosine between incident and medium-facing normal
/// - object_ior: index of refraction of object
/// Returns:
/// - fresnel: reflection ray weight
#[inline]
pub(super) const fn fresnel(medium_cos: f32, object_ior: f32) -> f32 {
    let f0 = (object_ior - MEDIUM_IOR) / (object_ior + MEDIUM_IOR);
    let f0 = f0 * f0;
    let t = 1. - medium_cos;
    let t2 = t * t;
    f0 + (1. - f0) * t2 * t2 * t
}
