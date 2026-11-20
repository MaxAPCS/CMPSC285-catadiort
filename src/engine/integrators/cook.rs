use ultraviolet::{Vec3, Vec3x4};

use super::{Integrator, RECAST_EPSILON, whitted};
use crate::engine::{MEDIUM_IOR, World, ray, wider};
use crate::utils::{ggx_albedo, samplers, warp};

const AODIST: f32 = 1.;

pub struct Cook;

impl Integrator for Cook {
    #[inline]
    fn integrate(
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

        // Sample RNG
        let specdirsuv: [_; 4] = rays.sampler.next2().into();
        let aodirsuv = rays.sampler.next2();
        let aodirs: [_; 4] =
            warp::recenterx4(warp::sample_cosine_hemispherex4(aodirsuv), hits.norm).into();

        let flecdirs: [_; 4] = hits.reflect().into(); // for SIMD
        let mut out = [Vec3::zero(); 4];
        let wghs: [_; 4] = wghs.into();
        let hits: [_; 4] = hits.into();
        let samplers: [_; 4] = core::mem::take(&mut rays.sampler).into();
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

            let mut child = |nwgh: Vec3, dir: Vec3| {
                children.push((
                    idx,
                    wgh * nwgh,
                    hit.wpos + wider::copysign(hit.wnorm, dir) * RECAST_EPSILON,
                    dir,
                    samplers[i].clone(),
                ));
            };

            // Shade
            hit.norm = wider::copysign(hit.norm, -hit.wdir);
            let (norm, matr) = hit.shade(depth);
            hit.norm = wider::copysign(norm.normalized(), -hit.wdir);

            // Spawn Secondary Rays & Compute Transmitted Albedo
            let transalbedo = if matr.transmission == 0. {
                // Opaque Surfaces

                // Directional Absorbed-then-Reemitted Color
                let albedo_from = {
                    // BRDF Total Weight
                    let 总数 = ggx_albedo::ggx_albedo_average(matr.roughness, matr.albedo);

                    let mut denominv = Vec3::one() / (Vec3::one() - 总数);
                    denominv.apply(|c| if !c.is_finite() { 0. } else { c.recip() });

                    move |wdir: Vec3| {
                        // Entrance Direction
                        let 进方 = hit.norm.dot(hit.wdir);
                        // Exit Direction
                        let 出方 = hit.norm.dot(wdir).max(0.);
                        // BRDF Entrance Weight
                        let 进数 = ggx_albedo::ggx_albedo(matr.roughness, 进方, matr.albedo);
                        // BRDF Exit Weight
                        let 出数 = ggx_albedo::ggx_albedo(matr.roughness, 出方, matr.albedo);
                        // BRDF Normalized "Unused" Weight
                        let 省 = (Vec3::one() - 进数) * (Vec3::one() - 出数) * denominv;
                        // Ray Direction Albedo
                        matr.albedo * (1. - matr.metallic) * 省
                    }
                };

                if matr.specs() {
                    if matr.roughness == 0. {
                        // Smooth Gloss

                        // Calculate Reflection Weight
                        let flecwgh = matr.specular
                            + (Vec3::one() - matr.specular) * (1. - hit.norm.dot(hit.wdir)).powi(5);
                        // Reflection
                        child(flecwgh, flecdirs[i])
                    } else {
                        // Rough Gloss
                        let smoothness =
                            ((matr.roughness * matr.roughness).recip() * 2. - 2.).max(1.);

                        // Sample Specular Reflection Direction
                        let specflecdir = warp::recenter(
                            warp::sample_phong_lobe(specdirsuv[i], smoothness),
                            flecdirs[i],
                        );
                        if specflecdir.dot(hit.norm) > 0. {
                            // Calculate Specular Reflection Weight
                            let specflecwgh = matr.specular
                                + (Vec3::one() - matr.specular)
                                    * (1. - (hit.wdir + specflecdir).normalized().dot(hit.wdir))
                                        .powi(5);
                            // Specular Reflection
                            child(specflecwgh, specflecdir)
                        }
                    }
                }

                albedo_from(aodirs[i])
            } else {
                // Translucent Surfaces

                // Calculate Reflection Weight & Refraction Direction
                let entering = hit.wdir.dot(hit.wnorm) <= 0.;
                let (flecwgh, fracdir) =
                    whitted::refraction(hit.wdir, hit.norm, matr.ior, entering);
                // Calculate Refraction Weight
                let tint =
                    Vec3::one() - (Vec3::broadcast(matr.albedo.component_max()) - matr.albedo);
                let eta = if entering {
                    MEDIUM_IOR / matr.ior
                } else {
                    matr.ior / MEDIUM_IOR
                };
                let fracwgh = (1. - flecwgh) * tint * eta * eta;

                if matr.roughness == 0. {
                    // Smooth Dielectric

                    {
                        // Reflection
                        child(matr.specular * flecwgh, flecdirs[i])
                    }
                    if !fracdir.x.is_nan() {
                        // Refraction
                        child(matr.transmission * fracwgh, fracdir)
                    }
                } else {
                    // Rough Dielectric
                    let smoothness = ((matr.roughness * matr.roughness).recip() * 2. - 2.).max(1.);

                    {
                        // Sample Specular Reflection Direction
                        let specflecdir = warp::recenter(
                            warp::sample_phong_lobe(specdirsuv[i], smoothness),
                            flecdirs[i],
                        );
                        if specflecdir.dot(hit.norm) > 0. {
                            // Specular Reflection
                            child(matr.specular * flecwgh, specflecdir)
                        }
                    }
                    if !fracdir.x.is_nan() {
                        // Sample Specular Refraction Direction
                        let specfracdir = warp::recenter(
                            warp::sample_phong_lobe(specdirsuv[i], smoothness),
                            fracdir,
                        );
                        if specfracdir.dot(hit.norm) <= 0. {
                            // Specular Refraction
                            child(matr.transmission * fracwgh, specfracdir)
                        }
                    }
                }

                (1. - matr.transmission) * matr.albedo
            };

            // Compute Ambient Occlusion Color
            let ambientcoeff = if AODIST > 0. && {
                let mut ar = ray::ShadowRay::new(
                    hit.wpos + wider::copysign(hit.wnorm * RECAST_EPSILON, aodirs[i]),
                    aodirs[i],
                    AODIST,
                    Vec3::one(),
                );
                world.obstruct(&mut ar, u16::MAX);
                ar.obstructed()
            } {
                Vec3::zero()
            } else {
                world.ambientcolor
            };

            // Compute Caustic Emission Factor
            let emissioncoeff = 1.; // TODO

            // Compute Weighted Final Color
            out[i] = wgh * (ambientcoeff * transalbedo + emissioncoeff * matr.emission)
        }
        ((*idxs, out), children)
    }
}
