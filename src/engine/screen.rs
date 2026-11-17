use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use rayon::slice::ParallelSlice;
use ultraviolet::{Vec3, Vec3x4};
use wide::f32x4;

use super::{ray, wider, world};
#[cfg(debug_assertions)]
use crate::rt_trace;
#[cfg(debug_assertions)]
use crate::utils::debug::vec3_display;

const RECAST_EPSILON: f32 = 1e-4;

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
        #[cfg(debug_assertions)] width: u32,
    ) {
        debug_assert!(!src.is_empty() && dst.is_empty());

        // Process Intersections
        src.par_iter_mut().for_each(|(_, _, rays)| {
            world.intersect(rays, depth);
            rays.collapse();
        });

        let mut contribs = Vec::with_capacity(src.len());
        let mut children = Vec::with_capacity(src.len() / 2);
        src.into_par_iter()
            .map(|(idxs, wghs, rays)| {
                let mut children = smallvec::SmallVec::<[_; 12]>::new();
                let Some(hits) = rays.get() else {
                    return (
                        (*idxs, (*wghs * world.sample_environmentx4(rays.dir)).into()),
                        children,
                    );
                };
                let mut out = [Vec3::zero(); 4];
                let wghs: [Vec3; 4] = (*wghs).into();
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

                    #[cfg(debug_assertions)]
                    {
                        rt_trace!(
                            idx / width,
                            idx % width,
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
            })
            .unzip_into_vecs(&mut contribs, &mut children);

        // Write Colors
        contribs.into_iter().for_each(|(idxs, contribs)| {
            for i in 0..4 {
                frame[idxs[i] as usize] += contribs[i];
            }
        });

        // Serialize Children
        let children: Vec<_> = children.into_par_iter().flatten_iter().collect();
        let children = children.par_chunks_exact(4);
        let rem = children.remainder();
        *dst = children
            .map(|chunk| {
                (
                    core::array::from_fn(|i| chunk[i].0),
                    core::array::from_fn(|i| chunk[i].1).into(),
                    ray::Rayx4::new(
                        core::array::from_fn(|i| chunk[i].2).into(),
                        core::array::from_fn(|i| chunk[i].3).into(),
                    ),
                )
            })
            .collect();
        if !rem.is_empty() {
            dst.push((
                core::array::from_fn(|i| rem.get(i).map_or_default(|r| r.0)),
                core::array::from_fn(|i| rem.get(i).map_or_default(|r| r.1)).into(),
                ray::Rayx4::new(
                    core::array::from_fn(|i| rem.get(i).map_or_default(|r| r.2)).into(),
                    core::array::from_fn(|i| rem.get(i).map_or_default(|r| r.3)).into(),
                ),
            ));
        }

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
                    let color: [Vec3; 4] = hits.color.into();
                    let t = hits.t.as_array_ref();
                    for i in 0..4 {
                        if color[i].x.is_nan() && !t[i].is_nan() {
                            out[i] = hits.material[i].as_deref().map_or(
                                const { crate::api::RtMaterial::default_material().albedo },
                                |m| m.prop.albedo,
                            );
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
