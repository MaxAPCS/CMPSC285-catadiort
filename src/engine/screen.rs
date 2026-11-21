use core::ops::Range;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefMutIterator, ParallelBridge,
    ParallelIterator,
};
use rayon::slice::ParallelSliceMut;
use ultraviolet::{Vec3, Vec3x4};
use wide::f32x4;

use super::{integrators::Integrator, ray, wider, world};
use crate::utils::{samplers::AnySamplerx4, warp};

type RayBuf = Vec<([u32; 4], Vec3x4, ray::Rayx4)>;

pub(crate) struct Screen {
    frame: Box<[Vec3]>,

    raysone: RayBuf,
    raystwo: RayBuf,
}

impl Screen {
    /// Arguments:
    /// - frame: persistent frame
    /// - focusdst: focal plane distance, scene units
    /// - lensrad: lens radius, ??? units
    /// - res: (width, height, sqrt(samples))
    /// - hslice: range of heights
    /// - seed: sampler pcg32 seed
    pub fn create(
        sampler: crate::options::Sampler,
        frame: Box<[Vec3]>,
        eyepos: Vec3,
        tofwd: Vec3,
        totop: Vec3,
        torgh: Vec3,
        focusdst: f32,
        lensrad: f32,
        res: (u32, u32, u32),
        hslice: Range<u32>,
        seed: u64,
    ) -> Self {
        debug_assert!((res.0 * hslice.len() as u32 * res.2 * res.2) % 4 == 0);
        debug_assert_eq!(frame.len() as u32, res.0 * res.1);

        let eyepos = Vec3x4::splat(eyepos);
        let tofwd = Vec3x4::splat(tofwd);
        let totop = Vec3x4::splat(totop);
        let torgh = Vec3x4::splat(torgh);

        let (wi, hi) = (
            f32x4::splat((res.0 as f32).recip()),
            f32x4::splat((res.1 as f32).recip()),
        );
        #[cfg(feature = "gaussian_weights")]
        let weights = wider::gaussian_weights(res.2 as _);
        #[cfg(not(feature = "gaussian_weights"))]
        let weights = Vec3x4::broadcast(f32x4::splat(((res.2 * res.2) as f32).recip()));

        Self {
            frame,
            raysone: wider::Chunku32x4::from(hslice.flat_map(|j| {
                (0..res.0).flat_map(move |i| {
                    (0..res.2).flat_map(move |nj| (0..res.2).map(move |ni| [i, j, ni, nj]))
                })
            }))
            .par_bridge()
            .map(|[i, j, ni, nj]| {
                let n = nj * res.2 + ni;
                let mut rayspl = AnySamplerx4::pixel(i, j, n, (res.2 * res.2) as _, seed, sampler);
                let film = rayspl.next2();
                let x = (wider::cast_f32x4(i) + film.x) * wi;
                let y = (wider::cast_f32x4(j) + film.y) * hi;

                let mut raysrc = eyepos;
                let mut raydir =
                    (tofwd + (2. * x - 1.) * torgh + (1. - 2. * y) * totop).normalized();

                let lens = warp::sample_concentric_diskx4(rayspl.next2());
                if lensrad > 0. {
                    raysrc = eyepos + (torgh * lens.x + totop * lens.y) * f32x4::splat(lensrad);
                    let lensdst = eyepos + raydir * (focusdst / (raydir.dot(tofwd)));
                    raydir = (lensdst - raysrc).normalized();
                }

                let _time = rayspl.next1();

                (
                    (j * res.0 + i).into(),
                    #[cfg(feature = "gaussian_weights")]
                    Vec3x4::broadcast(n.as_array_ref().map(|n| weights[n as usize]).into()),
                    #[cfg(not(feature = "gaussian_weights"))]
                    weights,
                    ray::Rayx4::new(raysrc, raydir, rayspl),
                )
            })
            .collect(),
            raystwo: Vec::new(),
        }
    }

    #[allow(private_bounds)]
    pub fn process<I: Integrator>(&mut self, integrator: I, world: &world::World, max_depth: u16) {
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
                integrator,
                &mut self.frame,
                &mut self.raysone,
                &mut self.raystwo,
                world,
                depth,
            );
            if self.raystwo.is_empty() || depth == max_depth {
                break;
            }
            Self::_pass(
                integrator,
                &mut self.frame,
                &mut self.raystwo,
                &mut self.raysone,
                world,
                depth + 1,
            );
            if self.raysone.is_empty() {
                break;
            }
        }
    }

    pub fn write(self) -> Box<[Vec3]> {
        self.frame
    }

    fn _pass<I: Integrator>(
        integrator: I,
        frame: &mut Box<[Vec3]>,
        src: &mut RayBuf,
        dst: &mut RayBuf,
        world: &world::World,
        depth: u16,
    ) {
        debug_assert!(!src.is_empty() && dst.is_empty());

        // Process Intersections
        src.par_iter_mut().for_each(|(_, _, rays)| {
            world.intersect(rays, depth);
            rays.collapse();
        });

        // Run Integrator
        let mut contribs = Vec::with_capacity(src.len());
        let mut children = Vec::with_capacity(src.len() / 2);
        src.into_par_iter()
            .map(|(idxs, wghs, rays)| integrator.integrate(&world, idxs, *wghs, rays, depth))
            .unzip_into_vecs(&mut contribs, &mut children);

        // Write Colors
        contribs.into_iter().for_each(|(idxs, contribs)| {
            for i in 0..4 {
                frame[idxs[i] as usize] += contribs[i];
            }
        });

        // Serialize Children
        let mut children: Vec<_> = children.into_par_iter().flatten_iter().collect(); // todo use accumx4 instead
        let mut children = children.par_chunks_exact_mut(4);
        let rem = children.remainder();
        let last = if !rem.is_empty() {
            Some((
                core::array::from_fn(|i| rem.get(i).map_or_default(|r| r.0)),
                core::array::from_fn(|i| rem.get(i).map_or_default(|r| r.1)).into(),
                ray::Rayx4::new(
                    core::array::from_fn(|i| rem.get(i).map_or_default(|r| r.2)).into(),
                    core::array::from_fn(|i| rem.get(i).map_or_default(|r| r.3)).into(),
                    core::array::from_fn(|i| {
                        rem.get_mut(i).map_or_default(|r| core::mem::take(&mut r.4))
                    })
                    .into(),
                ),
            ))
        } else {
            None
        };
        *dst = children
            .map(|chunk| {
                (
                    core::array::from_fn(|i| chunk[i].0),
                    core::array::from_fn(|i| chunk[i].1).into(),
                    ray::Rayx4::new(
                        core::array::from_fn(|i| chunk[i].2).into(),
                        core::array::from_fn(|i| chunk[i].3).into(),
                        core::array::from_fn(|i| core::mem::take(&mut chunk[i].4)).into(),
                    ),
                )
            })
            .collect();
        if let Some(last) = last {
            dst.push(last);
        }

        // Finish Pass
        #[cfg(debug_assertions)]
        println!(
            "screen: pass count src {} dst {}",
            src.len() * 4,
            dst.len() * 4
        );
        src.clear();
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
