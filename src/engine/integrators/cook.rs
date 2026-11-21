use ultraviolet::{Vec3, Vec3x4};

use super::{Integrator, RECAST_EPSILON, whitted};
use crate::engine::{MEDIUM_IOR, World, ray, wider};
use crate::utils::{ggx_albedo, samplers, warp};

#[derive(Clone, Copy)]
pub struct Cook {
    pub ao_distance: f32,
    pub lightsamples: u16,
}

impl Integrator for Cook {
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
        let wghs: [_; 4] = wghs.into();
        let hits: [_; 4] = hits.into();
        let mut samplers: [_; 4] = core::mem::take(&mut rays.sampler).into();
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
            hit.norm = wider::copysign(norm.normalized(), -hit.wdir);

            // Sample RNG
            let lightrays = world.lightrays(&hit.wpos, &mut samplers[i], self.lightsamples);
            let specdiruv = samplers[i].next2();
            let aodir = warp::recenter(
                warp::sample_cosine_hemisphere(samplers[i].next2()),
                hit.norm,
            );

            // Global Computations
            let entering = hit.wdir.dot(hit.wnorm) <= 0.;
            let flecdir = hit.reflect();
            let wnormf = wider::copysign(hit.wnorm, -hit.wdir);
            let alpha = matr.roughness * matr.roughness;
            let smoothness = ((alpha * alpha).recip() * 2. - 2.).max(1.);

            // NEE: Cast Shadow Rays
            let neecolor = {
                // Directional Absorbed-then-Reemitted Weight ( k_d / PI * cos_jk )
                let albedo: Box<dyn Fn(Vec3) -> Vec3> = {
                    if matr.transmission == 0. {
                        // Opaque

                        if matr.specs() {
                            // Coated

                            if alpha < 0.001 {
                                // Smooth Conductor

                                let 进方 = hit.norm.dot(-hit.wdir);
                                let 进数 = matr.specular
                                    + (Vec3::one() - matr.specular) * (1. - 进方).powi(5);
                                let 总数 = matr.specular + (Vec3::one() - matr.specular) / 21.;

                                let mut denominv = Vec3::one() / (Vec3::one() - 总数);
                                denominv.apply(|c| if c.is_finite() { c } else { 0. });

                                Box::new(move |wdir: Vec3| {
                                    let 出方 = hit.norm.dot(wdir).max(0.);
                                    let 出数 = matr.specular
                                        + (Vec3::one() - matr.specular) * (1. - 出方).powi(5);
                                    let 省 = (Vec3::one() - 进数) * (Vec3::one() - 出数) * denominv;
                                    matr.albedo * (1. - matr.metallic) * 省
                                })
                            } else {
                                // Rough Conductor

                                // Entrance Direction
                                let 进方 = hit.norm.dot(-hit.wdir);
                                // BRDF Entrance Weight
                                let 进数 =
                                    ggx_albedo::ggx_albedo(matr.roughness, 进方, matr.specular);
                                // BRDF Total Weight
                                let 总数 =
                                    ggx_albedo::ggx_albedo_average(matr.roughness, matr.specular);

                                let mut denominv = Vec3::one() / (Vec3::one() - 总数);
                                denominv.apply(|c| if c.is_finite() { c } else { 0. });

                                Box::new(move |wdir: Vec3| {
                                    // Exit Direction
                                    let 出方 = hit.norm.dot(wdir).max(0.);
                                    // BRDF Exit Weight
                                    let 出数: Vec3 =
                                        ggx_albedo::ggx_albedo(matr.roughness, 出方, matr.specular);
                                    // BRDF Normalized "Unused" Weight
                                    let 省 = (Vec3::one() - 进数) * (Vec3::one() - 出数) * denominv;
                                    // Ray Direction Albedo
                                    matr.albedo * (1. - matr.metallic) * 省
                                })
                            }
                        } else {
                            // Uncoated

                            Box::new(|_| matr.albedo * (1. - matr.metallic))
                        }
                    } else {
                        // Translucent
                        let r0 = ((matr.ior - MEDIUM_IOR) / (matr.ior + MEDIUM_IOR)).powi(2);

                        // Interface Entrance Weight
                        let 进数 = whitted::refraction(hit.wdir, hit.norm, matr.ior, entering).0;
                        // Interface Total Weight
                        let 总数 = if entering {
                            r0 + (1. - r0) / 21.
                        } else {
                            1. - 20. / 21. * (1. - r0) / (matr.ior * matr.ior)
                        };

                        let mut denominv = (1. - 总数).recip();
                        if !denominv.is_finite() {
                            denominv = 0.;
                        }

                        Box::new(move |wdir: Vec3| {
                            // Interface Exit Weight
                            let 出数 = whitted::refraction(-wdir, hit.norm, matr.ior, entering).0;
                            // Interface Normalized "Unused" Weight
                            let 省 = (1. - 进数) * (1. - 出数) * denominv;
                            // Ray Direction Albedo
                            matr.albedo * (1. - matr.metallic) * 省
                        })
                    }
                };

                let coslobe = (-smoothness.recip()).exp2();
                let mut specular = Vec3::zero();
                let mut diffuse = Vec3::zero();
                for (mut lr, car) in lightrays {
                    if lr.obstructed() || hit.norm.dot(lr.dir) <= 0. || wnormf.dot(lr.dir) <= 0. {
                        continue;
                    }
                    let lpos = lr.pos + lr.dir * lr.max_t;
                    lr.pos += wider::copysign(wnormf, lr.dir) * RECAST_EPSILON;
                    if lr.max_t.is_finite() {
                        let ldir = lpos - lr.pos;
                        let ldst = ldir.mag();
                        lr.dir = ldir / ldst;
                        lr.max_t = ldst - RECAST_EPSILON;
                    }
                    world.obstruct(&mut lr, u16::MAX);
                    if lr.obstructed() {
                        continue;
                    }
                    diffuse += albedo(lr.dir)
                        * core::f32::consts::FRAC_1_PI
                        * hit.norm.dot(lr.dir)
                        * lr.irradiance;
                    let cosout = flecdir.dot(lr.dir);
                    if matr.roughness > 0. && matr.specs() && car >= coslobe && cosout >= 0. {
                        let cosh = (-hit.wdir + lr.dir).normalized().dot(-hit.wdir);
                        let flecwgh = if matr.transmission == 0. {
                            matr.specular + (Vec3::one() - matr.specular) * (1. - cosh).powi(5)
                        } else {
                            matr.specular * whitted::fresnel(cosh, matr.ior)
                        };

                        let prob = (smoothness + 1.) * core::f32::consts::FRAC_1_PI / 2.
                            * cosout.powf(smoothness);
                        specular += flecwgh * prob * lr.irradiance;
                    }
                }
                (1. - matr.transmission) * diffuse + specular
            };

            // TODO specular rays should ignore consumed NEE emission

            let mut child = |nwgh: Vec3, dir: Vec3| {
                children.push((
                    idx,
                    wgh * nwgh,
                    hit.wpos + wider::copysign(hit.wnorm, dir) * RECAST_EPSILON,
                    dir,
                    samplers[i].clone(),
                ));
            };

            // Spawn Secondary Rays & Compute Transmitted Albedo ( 1-k_t * 1-metal * k_d * 1-Emu_0 )
            let transalbedo = if matr.transmission == 0. {
                // Opaque Surfaces

                if matr.specs() {
                    // Coated

                    // Compute Reflection Weight
                    let flecwgh = matr.specular
                        + (Vec3::one() - matr.specular) * (1. - hit.norm.dot(-hit.wdir)).powi(5);

                    if matr.roughness == 0. {
                        // Smooth Conductor

                        // Reflection
                        child(flecwgh, flecdir);

                        (1. - matr.metallic) * matr.albedo * (Vec3::one() - flecwgh)
                    } else {
                        // Rough Conductor

                        // Sample Specular Reflection Direction
                        let specflecdir =
                            warp::recenter(warp::sample_phong_lobe(specdiruv, smoothness), flecdir)
                                .normalized();
                        if specflecdir.dot(hit.norm) > 0. {
                            // Compute Specular Reflection Weight
                            let specflecwgh = matr.specular
                                + (Vec3::one() - matr.specular)
                                    * (1. - (-hit.wdir + specflecdir).normalized().dot(-hit.wdir))
                                        .powi(5);
                            // Specular Reflection
                            child(specflecwgh, specflecdir)
                        }

                        if alpha < 0.001 {
                            // Pseudosmooth Conductor

                            (1. - matr.metallic) * matr.albedo * (Vec3::one() - flecwgh)
                        } else {
                            // Rough Conductor

                            // Entrance Direction
                            let 进方 = hit.norm.dot(-hit.wdir);
                            // BRDF Entrance Weight
                            let 进数 = ggx_albedo::ggx_albedo(matr.roughness, 进方, matr.specular);
                            // BRDF Total Weight
                            let 总数 =
                                ggx_albedo::ggx_albedo_average(matr.roughness, matr.specular);

                            let mut coat = Vec3::one() - 进数;
                            let coatmut = coat.as_mut_array();
                            let mask: [_; 3] = (Vec3::one() - 总数).into();
                            for i in 0..3 {
                                if mask[i] == 0. {
                                    coatmut[i] = 0.
                                }
                            }

                            (1. - matr.metallic) * matr.albedo * coat
                        }
                    }
                } else {
                    // Uncoated

                    (1. - matr.metallic) * matr.albedo
                }
            } else {
                // Translucent Surfaces

                // Compute Reflection Weight & Refraction Direction
                let (flecwgh, fracdir) =
                    whitted::refraction(hit.wdir, hit.norm, matr.ior, entering);
                // Compute Refraction Weight
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
                        child(matr.specular * flecwgh, flecdir)
                    }
                    if !fracdir.x.is_nan() {
                        // Refraction
                        child(matr.transmission * fracwgh, fracdir)
                    }
                } else {
                    // Rough Dielectric

                    {
                        // Sample Specular Reflection Direction
                        let specflecdir =
                            warp::recenter(warp::sample_phong_lobe(specdiruv, smoothness), flecdir)
                                .normalized();
                        if specflecdir.dot(hit.norm) > 0. {
                            // Specular Reflection
                            child(matr.specular * flecwgh, specflecdir)
                        }
                    }
                    if !fracdir.x.is_nan() {
                        // Sample Specular Refraction Direction
                        let specfracdir =
                            warp::recenter(warp::sample_phong_lobe(specdiruv, smoothness), fracdir)
                                .normalized();
                        if specfracdir.dot(hit.norm) <= 0. {
                            // Specular Refraction
                            child(matr.transmission * fracwgh, specfracdir)
                        }
                    }
                }

                (1. - matr.transmission) * (1. - matr.metallic) * matr.albedo * (1. - flecwgh)
            };

            // Compute Ambient Occlusion Color (k_a * V_ao)
            let ambientcolor = if self.ao_distance > 0. && world.ambientcolor != Vec3::zero() && {
                let mut ar = ray::ShadowRay::new(
                    hit.wpos + wnormf * RECAST_EPSILON,
                    aodir,
                    self.ao_distance,
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
            out[i] = wgh * (ambientcolor * transalbedo + neecolor + emissioncoeff * matr.emission)
        }
        ((*idxs, out), children)
    }
}
