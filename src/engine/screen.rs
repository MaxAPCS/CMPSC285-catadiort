use rayon::{
    iter::{IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use ultraviolet::{Vec3, Vec3x4};
use wide::{CmpGt, f32x4};

use super::ray;
use super::wider;
use super::world;
use super::{RECAST_EPSILON, RECAST_EPSILON_X4};
#[cfg(debug_assertions)]
use crate::rt_trace;
#[cfg(debug_assertions)]
use crate::utils::debug::vec3_display;

type RayBuf = Vec<([u32; 4], Vec3x4, ray::Rayx4)>;
pub(crate) struct Screen {
    frame: Box<[Vec3]>,

    #[cfg(debug_assertions)]
    width: u32,

    raysone: RayBuf,
    raystwo: RayBuf,
}

impl Screen {
    /// Arguments:
    /// - eyepos: camera position
    /// - fwd: ray into screen, normalized
    /// - up: ray to screen top, normalized, perpendicular to fwd
    /// - fov: vertical field of view, radians
    pub fn create(eyepos: Vec3, fwd: Vec3, up: Vec3, fov: f32, screen: (u32, u32)) -> Self {
        debug_assert!(screen.0 * screen.1 % 4 == 0);
        let (w, h) = (screen.0 as f32, screen.1 as f32);
        let halfheight = (fov / 2.).tan();
        let tofwd = Vec3x4::splat(fwd);
        let totop = Vec3x4::splat(halfheight * up);
        let torgh = Vec3x4::splat(halfheight * (w / h) * Vec3::cross(&fwd, up));
        Self {
            frame: vec![Vec3::zero(); (screen.0 * screen.1) as usize].into_boxed_slice(),
            #[cfg(debug_assertions)]
            width: screen.0,
            raysone: wider::Chunku32x4::from(
                (0..screen.1).flat_map(|j| (0..screen.0).map(move |i| (i, j))),
            )
            .map(|(i, j)| {
                let x: f32x4 = (wider::cast_f32x4(i) + 0.5) / w;
                let y: f32x4 = (wider::cast_f32x4(j) + 0.5) / h;
                let dir = (tofwd + (2. * x - 1.) * torgh + (1. - 2. * y) * totop).normalized();
                #[cfg(debug_assertions)]
                {
                    let i = i.as_array_ref();
                    let j = j.as_array_ref();
                    let dir: [Vec3; 4] = dir.into();
                    for n in 0..4 {
                        rt_trace!(
                            i[n],
                            j[n],
                            "pixel ({}, {}): camera ray o=({}) d=({})",
                            i[n],
                            j[n],
                            vec3_display(eyepos),
                            vec3_display(dir[n])
                        );
                    }
                }
                (
                    (j * screen.0 + i).into(),
                    Vec3x4::one(),
                    ray::Rayx4::new(Vec3x4::splat(eyepos), dir),
                )
            })
            .collect(),
            raystwo: Vec::new(),
        }
    }

    pub fn process(&mut self, world: &world::World, max_depth: u16) {
        #[cfg(any(
            feature = "aovraydir",
            feature = "aovdepth",
            feature = "aovnormal",
            feature = "aovalbedo"
        ))]
        {
            self._aov(world);
            return;
        }
        for depth in (0..=max_depth).step_by(2) {
            Self::_pass(
                &mut self.frame,
                &mut self.raysone,
                &mut self.raystwo,
                world,
                depth,
                depth == max_depth,
                #[cfg(debug_assertions)]
                self.width,
            );
            if self.raystwo.is_empty() || depth == max_depth {
                break;
            }
            Self::_pass(
                &mut self.frame,
                &mut self.raystwo,
                &mut self.raysone,
                world,
                depth + 1,
                depth + 1 == max_depth,
                #[cfg(debug_assertions)]
                self.width,
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
        src: &mut RayBuf,
        dst: &mut RayBuf,
        world: &world::World,
        depth: u16,
        last_pass: bool,
        #[cfg(debug_assertions)] width: u32,
    ) {
        debug_assert!(!src.is_empty() && dst.is_empty());

        // Process Source Intersections
        src.par_iter_mut().for_each(|(_, _, rays)| {
            world.intersect(rays, depth);
            rays.collapse();
        });

        // Process Source Misses
        let srchits_est = src.len() * 3 / 4;
        let mut srchits = Vec::with_capacity(srchits_est);
        src.into_iter().for_each(|(idxs, wghs, rays)| {
            let wghs: [Vec3; 4] = (*wghs).into();
            let Some(hit) = rays.get() else {
                let env: [Vec3; 4] = world.sample_environmentx4(rays.dir).into();
                for i in 0..4 {
                    if wghs[i].component_max() <= 0. {
                        continue;
                    }
                    frame[idxs[i] as usize] += wghs[i] * env[i];
                }
                return;
            };
            let hits: [ray::Hit; 4] = (&hit).into();
            for (i, hit) in hits.into_iter().enumerate() {
                if wghs[i].component_max() <= 0. {
                    continue;
                }
                if hit.t.is_nan() {
                    frame[idxs[i] as usize] += wghs[i] * world.sample_environment(hit.wdir);
                    continue;
                }
                #[cfg(debug_assertions)]
                {
                    rt_trace!(
                        idxs[i] / width,
                        idxs[i] % width,
                        "depth {}: hit prim p t={} P=({}) N=({}) Ng=({}) {}",
                        depth,
                        hit.t,
                        vec3_display(hit.wpos),
                        vec3_display(hit.norm),
                        vec3_display(hit.wnorm),
                        if hit.wnorm.dot(hit.wdir) < 0. {
                            "entering"
                        } else {
                            "leaving"
                        }
                    );
                }
                srchits.push((idxs[i], wghs[i], hit));
            }
        });
        srchits.par_sort_unstable_by_key(|l| l.2.material);

        // Group Source Hits
        let srchitx4s_est = srchits.len() * 3 / 10;
        let mut srchitx4s: Vec<([u32; 4], Vec3x4, ray::Hitx4)> = Vec::with_capacity(srchitx4s_est);
        {
            let mut group: wider::Accum4<Vec3, ray::Hit> = wider::Accum4::new();
            for (idx, wgh, hit) in srchits {
                if let Some(peek) = group.peek()
                    && peek.2.material != hit.material
                {
                    let flush = unsafe { group.flush(Vec3::zero()).unwrap_unchecked() };
                    srchitx4s.push((flush.0, flush.1.into(), flush.2.into()));
                }
                if group.push(idx, wgh, hit) {
                    let flush = group.pop();
                    srchitx4s.push((flush.0, flush.1.into(), flush.2.into()));
                };
            }
            if let Some(flush) = group.flush(Vec3::zero()) {
                srchitx4s.push((flush.0, flush.1.into(), flush.2.into()));
            }
        }

        // Shade Source Hits
        srchitx4s.par_iter_mut().for_each(|(_, wghs, hits)| {
            let material = &world.materials[hits.material.as_array_ref()[0] as usize];

            hits.norm = wider::copysignx4(hits.norm, -hits.wdir);
            let (norm, matrs) = hits.shade(material, depth);
            hits.norm = wider::copysignx4(norm.normalized(), -hits.wdir); // write back shader norms

            let mut contribs = [Vec3::zero(); 4];
            let wnorm: [Vec3; 4] = hits.wnorm.into();
            let wpos: [Vec3; 4] = hits.wpos.into();
            let wdir: [Vec3; 4] = hits.wdir.into();
            let norm: [Vec3; 4] = hits.norm.into();
            let wghsa: [Vec3; 4] = (*wghs).into();

            for i in 0..4 {
                if wghsa[i].component_max() <= 0.0 {
                    continue;
                }
                let shadowrays = world.shadowrays(wpos[i]).filter_map(|mut sr| {
                    if sr.dir.dot(norm[i]) <= 0.0 {
                        return None;
                    }
                    sr.pos += wider::copysign(wnorm[i] * RECAST_EPSILON, sr.dir);
                    if world.shadow(&mut sr, depth) {
                        None
                    } else {
                        Some(sr)
                    }
                });
                contribs[i] = matrs[i].color(shadowrays, norm[i], wdir[i], world.ambientcolor);
                if wnorm[i].dot(wdir[i]) < 0. || !material.emits() {
                    contribs[i] += matrs[i].emission;
                }
            }
            let contribs: Vec3x4 = contribs.into();
            hits.color = contribs * *wghs; // write contribs into hit colors
        });

        // Write Source Colors
        srchitx4s.iter().for_each(
            |&(
                idxs,
                _,
                ray::Hitx4 {
                    color: contribs, ..
                },
            )| {
                let contribs: [Vec3; 4] = contribs.into();
                for i in 0..4 {
                    frame[idxs[i] as usize] += contribs[i];
                }
            },
        );

        if last_pass {
            return;
        }

        *dst = srchitx4s
            .into_par_iter()
            .flat_map_iter(|(idxs, wghs, hits)| {
                let material = &world.materials[hits.material.as_array_ref()[0] as usize];
                let mut children = smallvec::SmallVec::<[_; 3]>::new();
                if material.reflects() {
                    let nwgh = Vec3x4::splat(material.specular * material.smoothness());
                    let twgh = wghs * nwgh;

                    if twgh.component_max().cmp_gt(f32x4::ZERO).any() {
                        let reflectdir = hits.reflect();
                        children.push((
                            idxs,
                            twgh,
                            ray::Rayx4::new(
                                hits.wpos
                                    + wider::copysignx4(hits.wnorm, reflectdir) * RECAST_EPSILON_X4,
                                reflectdir,
                            ),
                        ))
                    }
                }
                if material.refracts() {
                    let (ndi, refractdir) = hits.refract(material);
                    let reflectwgh = f32x4::blend(
                        refractdir.x.is_nan(),
                        f32x4::ONE,
                        material.fresnelx4(ndi, material.ior.into()),
                    );
                    let reflectdir = hits.reflect();

                    children.push((
                        idxs,
                        wghs * Vec3x4::splat(material.specular) * Vec3x4::broadcast(reflectwgh),
                        ray::Rayx4::new(
                            hits.wpos
                                + wider::copysignx4(hits.wnorm, reflectdir) * RECAST_EPSILON_X4,
                            reflectdir,
                        ),
                    ));

                    if !refractdir.x.is_nan().all() {
                        children.push((
                            idxs,
                            wghs * (f32x4::ONE - reflectwgh) * f32x4::splat(material.transmission),
                            ray::Rayx4::new(
                                hits.wpos
                                    + wider::copysignx4(hits.wnorm, refractdir) * RECAST_EPSILON_X4,
                                refractdir,
                            ),
                        ));
                    }
                }
                children
            })
            .collect();

        #[cfg(debug_assertions)]
        println!("screen: pass count src {} dst {}", src.len(), dst.len());
        src.clear();
        dst.shrink_to_fit();
    }

    #[allow(unused_variables, unused_mut)]
    fn _aov(&mut self, world: &world::World) {
        self.raysone.par_iter_mut().for_each(|(_, _, rays)| {
            world.intersect(rays, 0);
            rays.collapse();
        });
        self.raysone.iter_mut().for_each(|(idxs, _, rays)| {
            let mut out = [Vec3::zero(); 4];
            #[cfg(feature = "aovraydir")]
            {
                out = (f32x4::HALF * rays.dir + Vec3x4::broadcast(f32x4::HALF)).into();
            }
            #[cfg(feature = "aovdepth")]
            {
                if let Some(hits) = rays.get() {
                    out = Vec3x4::broadcast(hits.t.cmp_gt(0.).blend(hits.t.recip(), f32x4::ZERO))
                        .into();
                }
            }
            #[cfg(feature = "aovnormal")]
            {
                if let Some(hits) = rays.get() {
                    out = (f32x4::HALF * hits.norm + Vec3x4::broadcast(f32x4::HALF)).into();
                }
            }
            #[cfg(feature = "aovalbedo")]
            {
                if let Some(hits) = rays.get() {
                    let material = hits.material.as_array_ref();
                    let color: [Vec3; 4] = hits.color.into();
                    let t = hits.t.as_array_ref();
                    for i in 0..4 {
                        if color[i].x.is_nan() && !t[i].is_nan() {
                            out[i] = (world.materials[material[i] as usize]).albedo.into();
                        } else {
                            out[i] = color[i];
                        }
                    }
                }
            }
            for i in 0..4 {
                self.frame[idxs[i] as usize] = out[i];
            }
        });
        self.raysone.clear();
    }
}
