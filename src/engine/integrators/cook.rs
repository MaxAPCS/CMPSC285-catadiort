use ultraviolet::{Vec3, Vec3x4};

use super::{Integrator, RECAST_EPSILON};
use crate::engine::{World, ray, wider};

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
        smallvec::SmallVec<[(u32, Vec3, Vec3, Vec3); 12]>,
    ) {
        let mut children = smallvec::SmallVec::<[_; 12]>::new();
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
            let shadowrays = world.shadowrays(hit.wpos).filter_map(|mut sr| {
                if sr.dir.dot(hit.norm) <= 0.0 || sr.obstructed() {
                    return None;
                }
                sr.pos += wider::copysign(hit.wnorm * RECAST_EPSILON, sr.dir);
                world.obstruct(&mut sr, u16::MAX);
                if sr.obstructed() { None } else { Some(sr) }
            });
            out[i] = wgh * matr.color(shadowrays, hit.norm, hit.wdir, world.ambientcolor);

            // Reflect
            if matr.reflects() {
                let reflectdir = hit.reflect();
                children.push((
                    idx,
                    wgh * matr.specular * matr.smoothness(),
                    hit.wpos + wider::copysign(hit.wnorm, reflectdir) * RECAST_EPSILON,
                    reflectdir,
                ))
            }

            // Refract
            if matr.refracts() {
                let (reflectwgh, refractdir) = hit.refract(matr.ior);
                let reflectdir = hit.reflect();
                children.push((
                    idx,
                    wgh * matr.specular * reflectwgh,
                    hit.wpos + wider::copysign(hit.wnorm, reflectdir) * RECAST_EPSILON,
                    reflectdir,
                ));
                if !refractdir.x.is_nan() {
                    children.push((
                        idx,
                        wgh * matr.transmission * (1. - reflectwgh),
                        hit.wpos + wider::copysign(hit.wnorm, refractdir) * RECAST_EPSILON,
                        refractdir,
                    ));
                }
            }
        }
        ((*idxs, out), children)
    }
}
