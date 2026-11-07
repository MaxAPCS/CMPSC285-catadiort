use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use ultraviolet::{Vec3, Vec3x4};
use wide::{CmpLe, f32x4};

use crate::utils::texture;

use super::object::{Primitive, Sphere};
use super::ray::{RayHit, Rayx4};
use super::util;
use super::world;
use super::{RECAST_EPSILON, RECAST_EPSILON_X4};

type RayBuf<'a> = Vec<([u32; 4], Vec3x4, Rayx4<'a>)>;
pub(crate) struct Screen<'a> {
    frame: Box<[Vec3]>,

    raysone: RayBuf<'a>,
    raystwo: RayBuf<'a>,
}

impl<'a> Screen<'a> {
    /// Arguments:
    /// - fwd: ray into screen, normalized
    /// - up: ray to screen top, normalized, perpendicular to fwd
    /// - fov: vertical field of angle, radians
    pub fn create(eyepos: Vec3, fwd: Vec3, up: Vec3, fov: f32, screen: (u32, u32)) -> Self {
        debug_assert!(screen.0 * screen.1 % 4 == 0);
        let (w, h) = (screen.0 as f32, screen.1 as f32);
        let halfheight = (fov / 2.).tan();
        let tofwd = Vec3x4::splat(fwd);
        let totop = Vec3x4::splat(halfheight * up);
        let tolft = Vec3x4::splat(halfheight * (w / h) * Vec3::cross(&fwd, up));
        Self {
            frame: vec![Vec3::zero(); (screen.0 * screen.1) as usize].into_boxed_slice(),
            raysone: util::Chunku32x4::from(
                (0..screen.1).flat_map(|j| (0..screen.0).map(move |i| (i, j))),
            )
            .map(|(i, j)| {
                let x: f32x4 = (util::cast_f32x4(i) + 0.5) / w;
                let y: f32x4 = (util::cast_f32x4(j) + 0.5) / h;

                (
                    (j * screen.0 + i).into(),
                    Vec3x4::one(),
                    Rayx4::new(
                        Vec3x4::splat(eyepos),
                        tofwd + (2. * x - 1.) * tolft + (1. - 2. * y) * totop,
                    ),
                )
            })
            .collect(),
            raystwo: Vec::new(),
        }
    }

    /// Run 2*floor(max_depth/2) passes
    pub fn process(&mut self, world: &'a world::World, max_depth: u16) {
        for depth in 0..max_depth / 2 {
            Self::_pass(
                &mut self.frame,
                &mut self.raysone,
                &mut self.raystwo,
                world,
                2 * depth,
            );
            if self.raystwo.is_empty() {
                break;
            }
            Self::_pass(
                &mut self.frame,
                &mut self.raystwo,
                &mut self.raysone,
                world,
                2 * depth + 1,
            );
            if self.raysone.is_empty() {
                break;
            }
        }
    }

    pub fn write(self) -> Box<[Vec3]> {
        self.frame
    }

    fn _pass(
        frame: &mut Box<[Vec3]>,
        src: &mut RayBuf<'a>,
        dst: &mut RayBuf,
        world: &'a world::World,
        depth: u16,
    ) {
        debug_assert!(dst.is_empty());

        src.par_iter_mut()
            .filter_map(|(idxs, weights, rays)| {
                world.intersect(rays, depth);
                rays.collapse();
                let Some(hits) = rays.get() else {
                    return Some((*idxs, world.sample_environmentx4(rays.dir).into()));
                };
                let weights: [Vec3; 4] = (*weights).into();
                let nclr: [Vec3; 4] = core::array::from_fn(|i| {
                    if weights[i].component_max() <= 0. {
                        return Vec3::zero();
                    }
                    if hits[i].t.is_nan() {
                        return world.sample_environment(hits[i].dir);
                    }
                    let (wnorm, mat) = hits[i].shade(depth);
                    #[cfg(feature = "aovnormal")]
                    {
                        return 0.5 * wnorm + Vec3::broadcast(0.5);
                    }
                    #[cfg(feature = "aovalbedo")]
                    {
                        return mat.albedo;
                    }
                    #[cfg(feature = "aovdepth")]
                    {
                        return if hits[i].t > 0.0 {
                            Vec3::broadcast(hits[i].t.recip())
                        } else {
                            Vec3::zero()
                        };
                    }
                    let shadowrays: Vec<_> = world
                        .shadowrays(hits[i].pos + wnorm * RECAST_EPSILON)
                        .filter_map(|mut sr| {
                            if sr.dir.dot(wnorm) <= 0. {
                                return None;
                            }
                            world.shadow(&mut sr, depth);
                            Some(sr)
                        })
                        .collect();
                    weights[i]
                        * mat.color(
                            shadowrays.as_slice(),
                            wnorm,
                            hits[i].dir,
                            true,
                            world.ambientcolor,
                        )
                });
                Some((*idxs, nclr))
            })
            .collect::<Box<_>>()
            .into_iter()
            .for_each(|(idxs, contribs)| {
                for i in 0..4 {
                    frame[idxs[i] as usize] += contribs[i]
                }
            });

        let dst_est = src.len() * 3 / 4;
        dst.reserve((dst_est).saturating_sub(dst.capacity()));

        let mut reflect = util::Accum4::new();
        let mut refract = util::Accum4::new();
        for (_, (idxs, weights, rays)) in src.iter().enumerate() {
            let Some(rhits) = rays.get() else { continue };
            let weights: [Vec3; 4] = (*weights).into();
            for (k, rhit) in rhits.into_iter().enumerate() {
                let obj = rhit.obj;
                if obj.refracts() && refract.push(idxs[k], weights[k], rhit.clone()) {
                    Self::_dorefractions(refract.pop(), dst);
                }
                if obj.reflects() && reflect.push(idxs[k], weights[k], rhit) {
                    Self::_doreflections(reflect.pop(), dst);
                }
            }
        }
        if let Some(reflect) = reflect.flush(Vec3::zero()) {
            Self::_doreflections(reflect, dst);
        }
        if let Some(refract) = refract.flush(Vec3::zero()) {
            Self::_dorefractions(refract, dst);
        }

        #[cfg(debug_assertions)]
        println!(
            "screen: pass count estimate {}, actual {}",
            dst_est,
            dst.len()
        );
        dst.shrink_to_fit();
        src.clear();
    }

    fn _doreflections(
        (idxs, owgh, hits): ([u32; 4], [Vec3; 4], [RayHit<Sphere>; 4]),
        dst: &mut RayBuf,
    ) {
        let owgh: Vec3x4 = owgh.into();
        let mut npos = [Vec3::zero(); 4];
        let mut odir = [Vec3::zero(); 4];
        let mut nwgh = [Vec3::zero(); 4];

        let mut sameobj = true;
        for (i, hit) in hits.iter().enumerate() {
            npos[i] = hit.pos;
            odir[i] = hit.dir;
            nwgh[i] = hit.obj.material.specular * hit.obj.material.smoothness();
            sameobj &= core::ptr::eq(hit.obj, hits[0].obj);
        }

        let npos = npos.into();
        let nwgh: Vec3x4 = nwgh.into();
        let reflectdir = if sameobj {
            hits[0].obj.reflectx4(npos, odir.into())
        } else {
            let mut reflectdir = [Vec3::zero(); 4];
            for (i, h) in hits.iter().enumerate() {
                reflectdir[i] = h.obj.reflect(h.pos, h.dir);
            }
            reflectdir.into()
        };

        let twgh = owgh * nwgh;
        if twgh.component_max().cmp_le(f32x4::ZERO).all() {
            return;
        }

        dst.push((
            idxs,
            twgh,
            Rayx4::new(
                npos + reflectdir.normalized() * RECAST_EPSILON_X4,
                reflectdir,
            ),
        ));
    }

    fn _dorefractions(
        (idxs, owgh, hits): ([u32; 4], [Vec3; 4], [RayHit<Sphere>; 4]),
        dst: &mut RayBuf,
    ) {
        let owgh: Vec3x4 = owgh.into();
        let mut npos = [Vec3::zero(); 4];
        let mut odir = [Vec3::zero(); 4];

        let mut sameobj = true;
        for (i, hit) in hits.iter().enumerate() {
            npos[i] = hit.pos;
            odir[i] = hit.dir;
            sameobj &= core::ptr::eq(hit.obj, hits[0].obj);
        }

        let npos = npos.into();

        let mut reflectwgh = [Vec3::zero(); 4];
        let mut refractdir = [Vec3::zero(); 4];
        let mut reflectdir = [Vec3::zero(); 4];
        if sameobj {
            let odir = odir.into();
            let obj = hits[0].obj;
            let (ndi, dir) = obj.refractx4(npos, odir);
            reflectwgh = Vec3x4::blend(
                dir.x.is_nan(),
                Vec3x4::zero(),
                Vec3x4::splat(obj.material.specular) * obj.material.fresnelx4(ndi),
            )
            .into();
            refractdir = dir.into();
            reflectdir = obj.reflectx4(npos, odir.into()).into();
        } else {
            for (i, h) in hits.iter().enumerate() {
                let (ndi, dir) = h.obj.refract(h.pos, h.dir);
                reflectwgh[i] = if dir.x.is_nan() {
                    Vec3::zero()
                } else {
                    h.obj.material.specular * h.obj.material.fresnel(ndi)
                };
                refractdir[i] = dir;
                reflectdir[i] = h.obj.reflect(h.pos, h.dir);
            }
        };
        let reflectwgh = reflectwgh.into();
        let refractdir: Vec3x4 = refractdir.into();
        let reflectdir = reflectdir.into();

        dst.push((
            idxs,
            owgh * reflectwgh,
            Rayx4::new(npos + reflectdir * RECAST_EPSILON_X4, reflectdir),
        ));
        if refractdir.x.is_nan().all() {
            return;
        }
        dst.push((
            idxs,
            owgh * (Vec3x4::one() - reflectwgh),
            Rayx4::new(npos - refractdir * RECAST_EPSILON_X4, refractdir),
        ));
    }
}
