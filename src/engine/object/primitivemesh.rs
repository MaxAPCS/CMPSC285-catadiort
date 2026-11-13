use bytemuck::Zeroable;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use std::sync::Arc;
use ultraviolet::{Mat4, Vec2, Vec2x4, Vec3, Vec3x4};
use wide::{CmpGe, CmpLe, f32x4, f64x4, u32x4};

use super::Primitive;
use super::ray;
use crate::api::{RtCutoutFn, RtMaterial, RtShadeContext};
#[cfg(feature = "bvh")]
use crate::engine::bvh::BVH;
use crate::engine::wider;

pub struct Mesh {
    wotrans: Mat4,
    owtrans: Mat4,

    cutout: RtCutoutFn,
    cutoutparams: Option<Arc<[u8]>>,
    material: u32,

    #[cfg(not(feature = "bvh"))]
    tris: Arc<[Tri]>,
    #[cfg(feature = "bvh")]
    tris: Arc<BVH<Tri>>,
}

pub(crate) struct MeshReference {
    pub owtrans: Mat4,

    pub cutout: RtCutoutFn,
    pub cutoutparams: Option<Arc<[u8]>>,
    pub material: u32,

    pub triref: usize,
}

impl Primitive for Mesh {
    fn bounds(&self) -> (Vec3, Vec3) {
        #[cfg(not(feature = "bvh"))]
        let (min, max) = self
            .tris
            .par_iter()
            .flat_map_iter(|tri| tri.oposs())
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
            );
        #[cfg(feature = "bvh")]
        let (min, max) = self.tris.bounds();
        (
            self.owtrans.transform_point3(min),
            self.owtrans.transform_point3(max),
        )
    }

    fn centroid(&self) -> Vec3 {
        #[cfg(not(feature = "bvh"))]
        let centroid = Self::tricentroid(&self.tris);
        #[cfg(feature = "bvh")]
        let centroid = self.tris.centroid();
        self.owtrans.transform_point3(centroid)
    }

    fn obstruct(&self, ray: &ray::ShadowRay, depth: u16) -> bool {
        let oray = ray::ShadowRay {
            pos: self.wotrans.transform_point3(ray.pos),
            dir: self.wotrans.transform_vec3(ray.dir),
            max_t: ray.max_t / self.owtrans.transform_vec3(ray.dir).mag(),
            irradiance: ray.irradiance,
        };
        #[cfg(not(feature = "bvh"))]
        {
            for triangle in self.tris.as_ref() {
                if triangle.obstruct(&oray, depth) {
                    return true;
                }
            }
            false
        }
        #[cfg(feature = "bvh")]
        self.tris.obstruct(&oray, depth)
    }

    fn intersect(&self, ray: &mut ray::Rayx4, depth: u16) {
        let mut oray = ray::Rayx4 {
            pos: wider::transform_pointx4(&self.wotrans, &ray.pos),
            dir: wider::transform_vecx4(&self.wotrans, ray.dir),
            hits: smallvec::SmallVec::new(),
        };
        #[cfg(not(feature = "bvh"))]
        {
            for triangle in self.tris.as_ref() {
                triangle.intersect(&mut oray, depth);
            }
        }
        #[cfg(feature = "bvh")]
        {
            self.tris.intersect(&mut oray, depth);
        }
        for mut hit in oray.hits {
            hit.wpos = ray.pos + ray.dir * hit.t;
            hit.wdir = ray.dir;
            hit.wnorm = wider::transform_vecx4(&self.wotrans.transposed(), hit.wnorm).normalized();
            hit.norm = wider::transform_vecx4(&self.wotrans.transposed(), hit.norm).normalized();
            hit.t = f32x4::blend(self.cutoutx4(&hit, depth), hit.t, wider::f32x4_NAN);
            hit.material = self.material.into();
            ray.hits.push(hit);
        }
    }
}

impl Mesh {
    pub fn tricreate(
        indexes: &[[i32; 3]],
        positions: Vec<Vec3>,
        normals: Option<Vec<Vec3>>,
        colors: Option<Vec<Vec3>>,
        uvs: Option<Vec<(f32, f32)>>,
    ) -> Box<[Tri]> {
        let details = Arc::new(MeshDetails {
            normals,
            colors,
            uvs,
        });
        indexes
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
            .collect()
    }

    pub fn tricentroid(triangles: &[Tri]) -> Vec3 {
        let (numer, denom) = triangles
            .par_iter()
            .fold(
                || (Vec3::zero(), 0.),
                |(numer, denom), tri| {
                    let area = tri.operp().mag();
                    (numer + tri.centroid() * area, denom + area)
                },
            )
            .reduce(|| (Vec3::zero(), 0.), |a, b| (a.0 + b.0, a.1 + b.1));
        numer / denom
    }

    pub(in crate::engine) fn create(
        meshref: MeshReference,
        #[cfg(not(feature = "bvh"))] tris: Arc<[Tri]>,
        #[cfg(feature = "bvh")] tris: Arc<BVH<Tri>>,
    ) -> Self {
        Self {
            wotrans: meshref.owtrans.inversed(),
            owtrans: meshref.owtrans,

            cutout: meshref.cutout,
            cutoutparams: meshref.cutoutparams,
            material: meshref.material,

            tris,
        }
    }

    fn cutout(&self, hit: &ray::Hit, depth: u16) -> bool {
        let Some(cutout) = self.cutout else {
            return true;
        };
        let mut ctx = RtShadeContext::new(
            &self.cutoutparams,
            hit.wpos,
            hit.wdir,
            hit.wnorm,
            hit.t,
            hit.opos,
            hit.norm,
            hit.uv,
            depth,
            RtMaterial::zeroed(), // todo: grab the actual material
        );
        (unsafe { cutout(&mut ctx) } != 0)
    }

    fn cutoutx4(&self, hit: &ray::Hitx4, depth: u16) -> f32x4 {
        let Some(cutout) = self.cutout else {
            return !f32x4::ZERO;
        };

        let opos: [Vec3; 4] = hit.opos.into();
        let wpos: [Vec3; 4] = hit.wpos.into();
        let wdir: [Vec3; 4] = hit.wdir.into();
        let wnorm: [Vec3; 4] = hit.wnorm.into();
        let t = hit.t.as_array_ref();
        let norm: [Vec3; 4] = hit.norm.into();
        let (u, v) = (hit.uv.0.as_array_ref(), hit.uv.1.as_array_ref());

        let mut out = [u32::MAX; 4];
        out.par_iter_mut().enumerate().for_each(|(i, out)| {
            let mut ctx = RtShadeContext::new(
                &self.cutoutparams,
                wpos[i],
                wdir[i],
                wnorm[i],
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
}

pub(crate) struct Tri {
    o: Vec3,
    e1: Vec3,
    e2: Vec3,

    idxs: [usize; 3],
    details: Arc<MeshDetails>,
}

struct MeshDetails {
    normals: Option<Vec<Vec3>>,
    colors: Option<Vec<Vec3>>,
    uvs: Option<Vec<(f32, f32)>>,
}

impl Primitive for Tri {
    fn bounds(&self) -> (Vec3, Vec3) {
        self.oposs().iter().fold(
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
                self.oposs()
                    .into_iter()
                    .reduce(|a, b| a + b)
                    .unwrap_unchecked()
            }
    }

    fn obstruct(&self, ray: &ray::ShadowRay, _: u16) -> bool {
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

        true
    }

    fn intersect(&self, rays: &mut ray::Rayx4, _: u16) {
        let o = wider::cast_d(Vec3x4::splat(self.o));
        let e1 = wider::cast_d(Vec3x4::splat(self.e1));
        let e2 = wider::cast_d(Vec3x4::splat(self.e2));
        let rwpos = wider::cast_d(rays.pos);
        let rwdir = wider::cast_d(rays.dir);

        let ray_cross_e2 = rwdir.cross(e2);
        let det = e1.dot(ray_cross_e2);
        let mut m = det.cmp_le(-f64::EPSILON) | det.cmp_ge(f64::EPSILON);
        if m.none() {
            return; // This ray is parallel to this triangle.
        }

        let inv_det = f64x4::ONE / det;
        let s = rwpos - o;
        let u = inv_det * s.dot(ray_cross_e2);
        m &= u.cmp_ge(f64x4::ZERO) & u.cmp_le(f64x4::ONE);
        if m.none() {
            return;
        }

        let s_cross_e1 = s.cross(e1);
        let v = inv_det * rwdir.dot(s_cross_e1);
        m &= v.cmp_ge(f64x4::ZERO) & (u + v).cmp_le(f64x4::ONE);
        if m.none() {
            return;
        }

        let u = wider::cast_f32(u);
        let v = wider::cast_f32(v);

        let uv = self.uvx4((u, v));
        let t = inv_det * e2.dot(s_cross_e1);
        m &= t.cmp_ge(f64x4::ZERO);
        if m.none() {
            return;
        }

        let t = wider::cast_f32(t);
        let m = wider::cast_m32(m);

        let opos = rays.pos + rays.dir * t;

        let onorm = Vec3x4::splat(self.onorm());
        rays.hits.push(ray::Hitx4 {
            opos,
            wpos: opos,
            wdir: rays.dir,
            wnorm: onorm,
            t: f32x4::blend(m, t, wider::f32x4_NAN),
            norm: self.normx4((u, v)).unwrap_or(onorm),
            uv,
            color: self
                .colorx4((u, v))
                .unwrap_or(Vec3x4::broadcast(wider::f32x4_NAN)),
            material: u32x4::MAX,
        })
    }
}

impl Tri {
    #[inline]
    fn oposs(&self) -> [Vec3; 3] {
        [self.o, self.o + self.e1, self.o + self.e2]
    }

    #[inline]
    fn onorms(&self) -> Option<[Vec3; 3]> {
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
    pub fn operp(&self) -> Vec3 {
        Vec3::cross(&self.e1, self.e2)
    }

    #[inline]
    pub fn onorm(&self) -> Vec3 {
        self.operp().normalized()
    }

    #[inline]
    fn norm(&self, bc: (f32, f32)) -> Option<Vec3> {
        self.onorms()
            .map(|normals| trilerp(bc, normals).normalized())
    }

    #[inline]
    fn normx4(&self, bc: (f32x4, f32x4)) -> Option<Vec3x4> {
        self.onorms()
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
