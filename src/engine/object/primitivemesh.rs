use bytemuck::Zeroable;
use rayon::{
    iter::{
        IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator,
        ParallelIterator,
    },
    slice::ParallelSliceMut,
};
use std::sync::Arc;
use ultraviolet::{Mat4, Vec2, Vec2x4, Vec3, Vec3x4};
use wide::{CmpGe, CmpLe, f32x4, f64x4, u32x4};

use super::Primitive;
use super::ray;
use crate::api::{RtCutoutFn, RtMaterial, RtShadeContext, WorldState};
use crate::engine::wider;

pub(crate) struct Mesh {
    pub hash: u64, // for instancing
    pub(super) triangles: Vec<Tri>,
}

struct MeshDetails {
    normals: Option<Vec<Vec3>>,
    colors: Option<Vec<Vec3>>,
    uvs: Option<Vec<(f32, f32)>>,

    wotrans: Mat4,

    cutout: RtCutoutFn,
    cutoutparams: Option<Arc<[u8]>>,
    pub material: usize,
}

impl Primitive for Mesh {
    fn bounds(&self) -> (Vec3, Vec3) {
        self.triangles
            .par_iter()
            .flat_map_iter(|tri| tri.positions())
            .fold(
                || {
                    const {
                        (
                            Vec3::broadcast(f32::INFINITY),
                            Vec3::broadcast(f32::NEG_INFINITY),
                        )
                    }
                },
                |a, x| {
                    (
                        Vec3::min_by_component(a.0, x),
                        Vec3::max_by_component(a.1, x),
                    )
                },
            )
            .reduce(
                || {
                    const {
                        (
                            Vec3::broadcast(f32::INFINITY),
                            Vec3::broadcast(f32::NEG_INFINITY),
                        )
                    }
                },
                |a, b| {
                    (
                        Vec3::min_by_component(a.0, b.0),
                        Vec3::max_by_component(a.1, b.1),
                    )
                },
            )
    }

    fn centroid(&self) -> Vec3 {
        let (numer, denom) = self
            .triangles
            .par_iter()
            .fold(
                || (Vec3::zero(), 0.),
                |(numer, denom), tri| {
                    let area = tri.wperp().mag();
                    (numer + tri.centroid() * area, denom + area)
                },
            )
            .reduce(|| (Vec3::zero(), 0.), |a, b| (a.0 + b.0, a.1 + b.1));
        numer / denom
    }

    #[allow(unused)]
    fn cutout(
        &self,
        wpos: Vec3,
        wdir: Vec3,
        t: f32,
        opos: Vec3,
        uv: (f32, f32),
        depth: u16,
    ) -> bool {
        unimplemented!("Mesh::cutout called instead of triangle delegation")
    }

    #[allow(unused)]
    fn cutoutx4(
        &self,
        wpos: Vec3x4,
        wdir: Vec3x4,
        t: f32x4,
        opos: Vec3x4,
        uv: (f32x4, f32x4),
        depth: u16,
    ) -> f32x4 {
        unimplemented!("Mesh::cutoutx4 called instead of triangle delegation")
    }

    fn obstruct(&self, ray: &ray::ShadowRay, depth: u16) -> bool {
        for triangle in &self.triangles {
            if triangle.obstruct(ray, depth) {
                return true;
            }
        }
        false
    }

    fn intersect(&self, rays: &ray::Rayx4, depth: u16) -> Option<ray::Hitx4> {
        let mut closest: Option<ray::Hitx4> = None;
        for triangle in &self.triangles {
            if let Some(hit) = triangle.intersect(rays, depth) {
                if let Some(closest) = closest.as_mut() {
                    closest.collapse(hit)
                } else {
                    closest = Some(hit)
                }
            }
        }
        closest
    }
}

impl Mesh {
    pub fn create(
        state: &WorldState,
        ctm: Mat4,
        hash: u64,
        indexes: &[[i32; 3]],
        mut positions: Vec<Vec3>,
        mut normals: Option<Vec<Vec3>>,
        colors: Option<Vec<Vec3>>,
        uvs: Option<Vec<(f32, f32)>>,
    ) -> Self {
        let mut piter = positions.par_chunks_exact_mut(4);
        for opos in piter.take_remainder() {
            *opos = ctm.transform_point3(*opos);
        }
        let owtrans = wider::Mat4x4_splat(&ctm);
        piter.for_each(|chunk| {
            let opos = Vec3x4::from(unsafe { *(chunk as *mut _ as *mut [Vec3; 4]) });
            let wpos = owtrans.transform_point3(opos);
            let mut wpos: [Vec3; 4] = wpos.into();
            chunk.swap_with_slice(&mut wpos);
        });
        let wotrans = ctm.inversed();
        if let Some(normals) = normals.as_mut() {
            let dual = wotrans.transposed();
            let mut niter = normals.par_chunks_exact_mut(4);
            for onorm in niter.take_remainder() {
                *onorm = dual.transform_vec3(*onorm).normalized();
            }
            let dual = wider::Mat4x4_splat(&dual);
            niter.for_each(|chunk| {
                let onorm = Vec3x4::from(unsafe { *(chunk as *mut _ as *mut [Vec3; 4]) });
                let wnorm = dual.transform_vec3(onorm).normalized();
                let mut wnorm: [Vec3; 4] = wnorm.into();
                chunk.swap_with_slice(&mut wnorm);
            });
        }
        let details = Arc::new(MeshDetails {
            normals,
            colors,
            uvs,

            wotrans,

            cutout: state.cutout,
            cutoutparams: state.cutoutparams.clone(),
            material: state.get_materialidx(),
        });
        Self {
            hash,
            triangles: indexes
                .into_iter()
                .map(move |idxs| {
                    let o = positions[idxs[0] as usize];
                    let o1 = positions[idxs[1] as usize];
                    let o2 = positions[idxs[2] as usize];
                    Tri {
                        o,
                        e1: o1 - o,
                        e2: o2 - o,
                        idxs: idxs.map(|idx| idx as _),
                        details: details.clone(),
                    }
                })
                .collect(),
        }
    }

    pub fn material(&self) -> usize {
        self.triangles[0].details.material
    }
}

pub(super) struct Tri {
    o: Vec3,
    e1: Vec3,
    e2: Vec3,

    idxs: [usize; 3],
    details: Arc<MeshDetails>,
}

impl Primitive for Tri {
    fn bounds(&self) -> (Vec3, Vec3) {
        self.positions().iter().fold(
            const {
                (
                    Vec3::broadcast(f32::INFINITY),
                    Vec3::broadcast(f32::NEG_INFINITY),
                )
            },
            |a, &x| {
                (
                    Vec3::min_by_component(a.0, x),
                    Vec3::max_by_component(a.1, x),
                )
            },
        )
    }

    fn centroid(&self) -> Vec3 {
        Vec3::broadcast(1. / 3.)
            * unsafe {
                self.positions()
                    .into_iter()
                    .reduce(|a, b| a + b)
                    .unwrap_unchecked()
            }
    }

    fn cutout(
        &self,
        wpos: Vec3,
        wdir: Vec3,
        t: f32,
        opos: Vec3,
        bc: (f32, f32),
        depth: u16,
    ) -> bool {
        let Some(cutout) = self.details.cutout else {
            return true;
        };

        let wnorm = self.wnorm();
        let norm = self.norm(bc).unwrap_or(wnorm);

        let mut ctx = RtShadeContext::new(
            &self.details.cutoutparams,
            wpos,
            wdir,
            wnorm,
            t,
            opos,
            norm,
            self.uv(bc),
            depth,
            RtMaterial::zeroed(), // todo: grab the actual material
        );
        (unsafe { cutout(&mut ctx) } != 0)
    }

    fn cutoutx4(
        &self,
        wpos: Vec3x4,
        wdir: Vec3x4,
        t: f32x4,
        opos: Vec3x4,
        bc: (f32x4, f32x4),
        depth: u16,
    ) -> f32x4 {
        let Some(cutout) = self.details.cutout else {
            return !f32x4::ZERO;
        };

        let wpos: [Vec3; 4] = wpos.into();
        let wdir: [Vec3; 4] = wdir.into();
        let wnorm = self.wnorm();
        let t = t.as_array_ref();
        let opos: [Vec3; 4] = opos.into();
        let norm: [Vec3; 4] = self.normx4(bc).unwrap_or(Vec3x4::splat(wnorm)).into();
        let uv = self.uvx4(bc);
        let (u, v) = (uv.0.as_array_ref(), uv.1.as_array_ref());
        let params = &self.details.cutoutparams;

        let mut out = [u32::MAX; 4];
        out.par_iter_mut().enumerate().for_each(|(i, out)| {
            let mut ctx = RtShadeContext::new(
                params,
                wpos[i],
                wdir[i],
                wnorm,
                t[i],
                opos[i],
                norm[i],
                (u[i], v[i]),
                depth,
                RtMaterial::zeroed(), // todo: grab the actual material
            );
            if unsafe { cutout(&mut ctx) } == 0 {
                *out = 0
            }
        });
        let out: u32x4 = out.into();
        bytemuck::cast(out)
    }

    fn obstruct(&self, ray: &ray::ShadowRay, depth: u16) -> bool {
        let ray_cross_e2 = ray.dir.cross(self.e2);
        let det = self.e1.dot(ray_cross_e2);
        if !(det <= -f32::EPSILON || det >= f32::EPSILON) {
            return false; // This ray is parallel to this triangle.
        }

        let inv_det = det.recip();
        let s = ray.pos - self.o;
        let u = inv_det * s.dot(ray_cross_e2);
        if !(u >= 0. && u <= 1.) {
            return false;
        }

        let s_cross_e1 = s.cross(self.e1);
        let v = inv_det * ray.dir.dot(s_cross_e1);
        if !(v >= 0. && (u + v) <= 1.) {
            return false;
        }

        let t = inv_det * self.e2.dot(s_cross_e1);
        if t <= 0. || t >= ray.max_t {
            return false;
        }

        let wpos = ray.pos + ray.dir * t;
        let opos = self.details.wotrans.transform_point3(wpos);
        self.cutout(wpos, ray.dir, t, opos, (u, v), depth)
    }

    fn intersect(&self, ray: &ray::Rayx4, depth: u16) -> Option<ray::Hitx4> {
        let o = wider::cast_d(Vec3x4::splat(self.o));
        let e1 = wider::cast_d(Vec3x4::splat(self.e1));
        let e2 = wider::cast_d(Vec3x4::splat(self.e2));
        let rwpos = wider::cast_d(ray.pos);
        let rwdir = wider::cast_d(ray.dir);

        let ray_cross_e2 = rwdir.cross(e2);
        let det = e1.dot(ray_cross_e2);
        let mut m = det.cmp_le(-f64::EPSILON) | det.cmp_ge(f64::EPSILON);
        if m.none() {
            return None; // This ray is parallel to this triangle.
        }

        let inv_det = f64x4::ONE / det;
        let s = rwpos - o;
        let u = inv_det * s.dot(ray_cross_e2);
        m &= u.cmp_ge(f64x4::ZERO) & u.cmp_le(f64x4::ONE);
        if m.none() {
            return None;
        }

        let s_cross_e1 = s.cross(e1);
        let v = inv_det * rwdir.dot(s_cross_e1);
        m &= v.cmp_ge(f64x4::ZERO) & (u + v).cmp_le(f64x4::ONE);
        if m.none() {
            return None;
        }

        let u = wider::cast_f32(u);
        let v = wider::cast_f32(v);

        let uv = self.uvx4((u, v));
        let t = inv_det * e2.dot(s_cross_e1);
        m &= t.cmp_ge(f64x4::ZERO);
        if m.none() {
            return None;
        }

        let t = wider::cast_f32(t);
        let mut m = wider::cast_m32(m);

        let wpos = ray.pos + ray.dir * t;
        let opos = wider::transform_pointx4(&self.details.wotrans, &wpos);
        m &= self.cutoutx4(wpos, ray.dir, t, opos, (u, v), depth);

        let wnorm = Vec3x4::splat(self.wnorm());
        Some(ray::Hitx4 {
            opos,
            wpos,
            wdir: ray.dir,
            wnorm,
            t: f32x4::blend(m, t, wider::f32x4_NAN),
            norm: self.normx4((u, v)).unwrap_or(wnorm),
            uv,
            color: self
                .colorx4((u, v))
                .unwrap_or(Vec3x4::broadcast(wider::f32x4_NAN)),
            material: (self.details.material as u32).into(),
        })
    }
}

impl Tri {
    #[inline]
    fn positions(&self) -> [Vec3; 3] {
        [self.o, self.o + self.e1, self.o + self.e2]
    }

    #[inline]
    fn normals(&self) -> Option<[Vec3; 3]> {
        self.details
            .normals
            .as_ref()
            .map(|normals| self.idxs.map(|idx| normals[idx].clone()))
    }

    #[inline]
    fn uvs(&self) -> Option<[(f32, f32); 3]> {
        self.details
            .uvs
            .as_ref()
            .map(|uvs| self.idxs.map(|idx| uvs[idx]))
    }

    #[inline]
    fn colors(&self) -> Option<[Vec3; 3]> {
        self.details
            .colors
            .as_ref()
            .map(|colors| self.idxs.map(|idx| colors[idx].clone()))
    }

    #[inline]
    pub fn wperp(&self) -> Vec3 {
        Vec3::cross(&self.e1, self.e2)
    }

    #[inline]
    pub fn wnorm(&self) -> Vec3 {
        self.wperp().normalized()
    }

    #[inline]
    fn norm(&self, bc: (f32, f32)) -> Option<Vec3> {
        self.normals()
            .map(|normals| trilerp(bc, normals).normalized())
    }

    #[inline]
    fn normx4(&self, bc: (f32x4, f32x4)) -> Option<Vec3x4> {
        self.normals()
            .map(|normals| trilerpx4(bc, normals.map(Vec3x4::splat)).normalized())
    }

    #[inline]
    fn uv(&self, bc: (f32, f32)) -> (f32, f32) {
        self.uvs().map_or(bc, |uvs| {
            trilerp(bc, uvs.map(|(u, v)| Vec2::new(u, v))).into()
        })
    }

    #[inline]
    fn uvx4(&self, bc: (f32x4, f32x4)) -> (f32x4, f32x4) {
        self.uvs().map_or(bc, |uvs| {
            trilerpx4(bc, uvs.map(|(u, v)| Vec2x4::new_splat(u, v))).into()
        })
    }

    #[inline]
    fn colorx4(&self, bc: (f32x4, f32x4)) -> Option<Vec3x4> {
        self.colors()
            .map(|colors| trilerpx4(bc, colors.map(Vec3x4::splat)))
    }
}

/// Arguments:
/// - bc: last two barycentric weights
#[inline]
fn trilerp<T: Copy + std::ops::Sub<Output = T> + std::ops::Add<Output = T>>(
    (b, c): (f32, f32),
    values: [T; 3],
) -> T
where
    f32: std::ops::Mul<T, Output = T>,
{
    values[0] + b * (values[1] - values[0]) + c * (values[2] - values[0])
}

/// Arguments:
/// - bc: last two barycentric weights
#[inline]
fn trilerpx4<T: Copy + std::ops::Sub<Output = T> + std::ops::Add<Output = T>>(
    (b, c): (f32x4, f32x4),
    values: [T; 3],
) -> T
where
    f32x4: std::ops::Mul<T, Output = T>,
{
    values[0] + b * (values[1] - values[0]) + c * (values[2] - values[0])
}
