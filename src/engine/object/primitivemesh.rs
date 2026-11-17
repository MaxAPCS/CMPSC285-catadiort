use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::sync::Arc;
use ultraviolet::{Mat4, Vec2, Vec2x4, Vec3, Vec3x4};
use wide::{CmpGe, CmpLe, f32x4, f64x4, u32x4};

use super::Primitive;
use crate::api::{RtCutoutFn, RtShadeContext};
#[cfg(feature = "bvh")]
use crate::engine::bvh::BVH;
use crate::engine::{Material, ray, wider};

pub struct Mesh {
    /// World to Object Transform
    wotrans: Mat4,
    /// Dual of Object to World Transform
    owtrans_dual: Mat4,

    /// Cutout Shader Reference
    cutout: RtCutoutFn,
    cutoutparams: Option<Arc<[u8]>>,

    /// Material Properties
    material: Arc<Material>,

    #[cfg(not(feature = "bvh"))]
    tris: Arc<[Tri]>,
    #[cfg(feature = "bvh")]
    tris: Arc<BVH<Tri>>,
}

pub(crate) struct MeshReference {
    pub owtrans: Mat4,

    pub cutout: RtCutoutFn,
    pub cutoutparams: Option<Arc<[u8]>>,
    pub material: Arc<Material>,

    pub triref: usize,
}

macro_rules! make_corners {
    ($x1:expr, $x2:expr, $y1:expr, $y2:expr, $z1:expr, $z2:expr) => {
        [
            Vec3::new($x1, $y1, $z1),
            Vec3::new($x2, $y1, $z1),
            Vec3::new($x1, $y2, $z1),
            Vec3::new($x2, $y2, $z1),
            Vec3::new($x1, $y1, $z2),
            Vec3::new($x2, $y1, $z2),
            Vec3::new($x1, $y2, $z2),
            Vec3::new($x2, $y2, $z2),
        ]
    };
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

        let mut wmin = Vec3::broadcast(f32::INFINITY);
        let mut wmax = Vec3::broadcast(f32::NEG_INFINITY);
        let owtrans = self.wotrans.inversed();
        for ocorn in make_corners!(min.x, max.x, min.y, max.y, min.z, max.z) {
            let wcorn = owtrans.transform_point3(ocorn);
            wmin = wmin.min_by_component(wcorn);
            wmax = wmax.max_by_component(wcorn);
        }
        (wmin, wmax)
    }

    #[inline]
    fn centroid(&self) -> Vec3 {
        #[cfg(not(feature = "bvh"))]
        let centroid = Self::tricentroid(&self.tris);
        #[cfg(feature = "bvh")]
        let centroid = self.tris.centroid();
        self.wotrans.inversed().transform_point3(centroid)
    }

    fn obstruct(&self, ray: &mut ray::ShadowRay, depth: u16) {
        let mut oray = ray::ShadowRay::new(
            self.wotrans.transform_point3(ray.pos),
            self.wotrans.transform_vec3(ray.dir),
            ray.max_t,
            ray.irradiance,
        );
        #[cfg(not(feature = "bvh"))]
        {
            for triangle in self.tris.as_ref() {
                triangle.obstruct(&mut oray, depth);
            }
        }
        #[cfg(feature = "bvh")]
        {
            self.tris.obstruct(&mut oray, depth);
        }
        for mut hit in oray.hits {
            hit.opos = hit.wpos;
            hit.wpos = ray.pos + ray.dir * hit.t;
            hit.wdir = ray.dir;
            hit.wnorm = self.owtrans_dual.transform_vec3(hit.wnorm).normalized();
            hit.norm = self.owtrans_dual.transform_vec3(hit.norm).normalized();
            hit.material = Some(self.material.clone());
            if !self.cutout(&hit, depth) {
                continue;
            }
            ray.hits.push(hit);
        }
    }

    fn intersect(&self, ray: &mut ray::Rayx4, depth: u16) {
        let mut oray = ray.transformed(&self.wotrans);
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
            hit.opos = hit.wpos;
            hit.wpos = ray.pos + ray.dir * hit.t;
            hit.wdir = ray.dir;
            hit.wnorm = wider::transform_vecx4(&self.owtrans_dual, hit.wnorm).normalized();
            hit.norm = wider::transform_vecx4(&self.owtrans_dual, hit.norm).normalized();
            hit.material = core::array::from_fn(|_| Some(self.material.clone()));
            hit.t = f32x4::blend(self.cutoutx4(&hit, depth), hit.t, wider::f32x4_NAN);
            if hit.t.is_nan().all() {
                continue;
            }
            ray.push(hit);
        }
    }
}

// Primitive Helper Functions
impl Mesh {
    #[inline]
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
            self.material.prop,
        );
        (unsafe { cutout(&mut ctx) } != 0)
    }

    #[inline]
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
        for i in 0..4 {
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
                self.material.prop,
            );
            if unsafe { cutout(&mut ctx) } == 0 {
                out[i] = 0
            }
        }
        let out: u32x4 = out.into();
        bytemuck::cast(out)
    }
}

impl Mesh {
    pub fn tricreate(
        indexes: &[[i32; 3]],
        positions: Vec<Vec3>,
        normals: Option<Vec<Vec3>>,
        colors: Option<Vec<Vec3>>,
        uvs: Option<Vec<[f32; 2]>>,
    ) -> Box<[Tri]> {
        let details = Arc::new(TriData {
            normals,
            colors,
            uvs,
        });
        indexes
            .into_par_iter()
            .map(move |idxs| {
                let o0 = positions[idxs[0] as usize];
                let o1 = positions[idxs[1] as usize];
                let o2 = positions[idxs[2] as usize];
                let e1 = o1 - o0;
                let e2 = o2 - o0;
                Tri {
                    o: o0,
                    e1,
                    e2,
                    onorm: Vec3::cross(&e1, e2).normalized(),
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

    #[inline]
    pub(in crate::engine) fn create(
        meshref: MeshReference,
        #[cfg(not(feature = "bvh"))] tris: Arc<[Tri]>,
        #[cfg(feature = "bvh")] tris: Arc<BVH<Tri>>,
    ) -> Self {
        let wotrans = meshref.owtrans.inversed();
        Self {
            wotrans,
            owtrans_dual: wotrans.transposed(),

            cutout: meshref.cutout,
            cutoutparams: meshref.cutoutparams,
            material: meshref.material,

            tris,
        }
    }
}

pub(crate) struct Tri {
    o: Vec3,
    e1: Vec3,
    e2: Vec3,
    pub onorm: Vec3,

    idxs: [usize; 3],
    details: Arc<TriData>,
}

struct TriData {
    normals: Option<Vec<Vec3>>,
    colors: Option<Vec<Vec3>>,
    uvs: Option<Vec<[f32; 2]>>,
}

impl Primitive for Tri {
    #[inline]
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

    #[inline]
    fn centroid(&self) -> Vec3 {
        unsafe {
            self.oposs()
                .into_iter()
                .reduce(|a, b| a + b)
                .unwrap_unchecked()
                / 3.
        }
    }

    fn obstruct(&self, ray: &mut ray::ShadowRay, _: u16) {
        let ray_cross_e2 = ray.dir.cross(self.e2);
        let det = self.e1.dot(ray_cross_e2);
        // no epsilon- shadows can be a little bigger, who cares
        if det == 0. {
            return; // This ray is parallel to this triangle.
        }
        let inv_det = det.recip();

        let s = ray.pos - self.o;
        let u = inv_det * s.dot(ray_cross_e2);
        if !(u >= 0. && u <= 1.) {
            return;
        }

        let s_cross_e1 = s.cross(self.e1);
        let v = inv_det * ray.dir.dot(s_cross_e1);
        if !(v >= 0. && (u + v) <= 1.) {
            return;
        }

        let t = inv_det * self.e2.dot(s_cross_e1);
        if t <= 0. || t >= ray.max_t {
            return;
        }

        ray.hits.push(ray::Hit {
            opos: Vec3::broadcast(f32::NAN),
            wpos: ray.pos + ray.dir * t,
            wdir: ray.dir,
            wnorm: self.onorm,
            t,
            norm: self.norm((u, v)).unwrap_or(self.onorm),
            uv: self.uv((u, v)),
            color: self.color((u, v)).unwrap_or(Vec3::broadcast(f32::NAN)),
            material: None,
        })
    }

    fn intersect(&self, ray: &mut ray::Rayx4, _: u16) {
        let o = wider::cast_d(Vec3x4::splat(self.o));
        let e1 = wider::cast_d(Vec3x4::splat(self.e1));
        let e2 = wider::cast_d(Vec3x4::splat(self.e2));
        let rwpos = wider::cast_d(ray.pos);
        let rwdir = wider::cast_d(ray.dir);

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

        let t = inv_det * e2.dot(s_cross_e1);
        m &= t.cmp_ge(f64x4::ZERO);
        if m.none() {
            return;
        }

        let uv = (wider::cast_f32(u), wider::cast_f32(v));
        let onorm = Vec3x4::splat(self.onorm);
        ray.hits.push(ray::Hitx4 {
            opos: Vec3x4::broadcast(wider::f32x4_NAN),
            wpos: wider::cast_s(rwpos + rwdir * t),
            wdir: ray.dir,
            wnorm: onorm,
            t: f32x4::blend(wider::cast_m32(m), wider::cast_f32(t), wider::f32x4_NAN),
            norm: self.normx4(uv).unwrap_or(onorm),
            uv: self.uvx4(uv),
            color: self
                .colorx4(uv)
                .unwrap_or(Vec3x4::broadcast(wider::f32x4_NAN)),
            material: [const { None }; 4],
        })
    }
}

impl Tri {
    #[inline]
    fn oposs(&self) -> [Vec3; 3] {
        [self.o, self.o + self.e1, self.o + self.e2]
    }

    #[inline]
    pub fn operp(&self) -> Vec3 {
        Vec3::cross(&self.e1, self.e2)
    }

    #[inline]
    fn norms(&self) -> Option<[Vec3; 3]> {
        self.details
            .normals
            .as_ref()
            .map(|normals| self.idxs.map(|idx| normals[idx]))
    }

    #[inline]
    fn norm(&self, bc: (f32, f32)) -> Option<Vec3> {
        self.norms()
            .map(|normals| trilerp(bc, normals).normalized())
    }

    #[inline]
    fn normx4(&self, bc: (f32x4, f32x4)) -> Option<Vec3x4> {
        self.norms()
            .map(|normals| trilerp(bc, normals.map(Vec3x4::splat)).normalized())
    }

    #[inline]
    fn uvs(&self) -> Option<[[f32; 2]; 3]> {
        self.details
            .uvs
            .as_ref()
            .map(|uvs| self.idxs.map(|idx| uvs[idx]))
    }

    #[inline]
    fn uv(&self, bc: (f32, f32)) -> (f32, f32) {
        self.uvs().map_or(bc, |uvs| {
            trilerp(bc, uvs.map(|[u, v]| Vec2::new(u, v))).into()
        })
    }

    #[inline]
    fn uvx4(&self, bc: (f32x4, f32x4)) -> (f32x4, f32x4) {
        self.uvs().map_or(bc, |uvs| {
            trilerp(bc, uvs.map(|[u, v]| Vec2x4::new_splat(u, v))).into()
        })
    }

    #[inline]
    fn colors(&self) -> Option<[Vec3; 3]> {
        self.details
            .colors
            .as_ref()
            .map(|colors| self.idxs.map(|idx| colors[idx]))
    }

    #[inline]
    fn color(&self, bc: (f32, f32)) -> Option<Vec3> {
        self.colors().map(|colors| trilerp(bc, colors))
    }

    #[inline]
    fn colorx4(&self, bc: (f32x4, f32x4)) -> Option<Vec3x4> {
        self.colors()
            .map(|colors| trilerp(bc, colors.map(Vec3x4::splat)))
    }
}

/// Arguments:
/// - bc: last two barycentric weights
#[inline]
fn trilerp<T, U>((b, c): (U, U), values: [T; 3]) -> T
where
    T: Copy + core::ops::Sub<Output = T> + core::ops::Add<Output = T>,
    U: core::ops::Mul<T, Output = T>,
{
    values[0] + b * (values[1] - values[0]) + c * (values[2] - values[0])
}
