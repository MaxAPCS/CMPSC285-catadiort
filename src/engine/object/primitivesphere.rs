use bytemuck::Zeroable;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use std::sync::Arc;
use ultraviolet::{Mat4, Vec3, Vec3x4};
use wide::{CmpGe, CmpLt, CmpNe, f32x4, f64x4, u32x4};

use super::Primitive;
use crate::api::{RtCutoutFn, RtMaterial, RtShadeContext};
use crate::engine::{Material, ray, wider};

pub struct Sphere {
    /// World to Object Transform
    wotrans: Mat4,

    /// Cutout Shader Reference
    cutout: RtCutoutFn,
    cutoutparams: Option<Arc<[u8]>>,

    /// Material Properties
    material: Arc<Material>,
}

impl Primitive for Sphere {
    #[inline]
    fn bounds(&self) -> (Vec3, Vec3) {
        let owtrans = self.wotrans.inversed();
        let center = owtrans.transform_point3(Vec3::zero());
        let owtrans = owtrans.truncate();
        let half = Vec3::new(
            owtrans.cols[0].mag(),
            owtrans.cols[1].mag(),
            owtrans.cols[2].mag(),
        );
        (center - half, center + half)
    }

    #[inline]
    fn centroid(&self) -> Vec3 {
        self.wotrans.inversed().extract_translation()
    }

    fn obstruct(&self, ray: &mut ray::ShadowRay, depth: u16) {
        let ropos = self.wotrans.transform_point3(ray.pos);
        let rodir = self.wotrans.transform_vec3(ray.dir);

        let a = rodir.mag_sq();
        let b = ropos.dot(rodir);
        let c = ropos.mag_sq() - 1.;
        let discrim = b * b - a * c;

        if discrim < 0. {
            return;
        }

        {
            let t1 = (-b - discrim.sqrt()) / a;
            if t1 >= ray.max_t {
                return;
            }
            let opos1 = ropos + rodir * t1;
            let wpos1 = ray.pos + ray.dir * t1;
            let wnorm1 = self.wotrans.transposed().transform_vec3(opos1).normalized();
            let uv1 = Self::uv(opos1);
            if (t1 > 0.) && self.cutout(wpos1, ray.dir, wnorm1, t1, opos1, uv1, depth) {
                ray.hits.push(ray::Hit {
                    opos: opos1,
                    wpos: wpos1,
                    wdir: ray.dir,
                    wnorm: wnorm1,
                    t: t1,
                    norm: wnorm1,
                    uv: uv1,
                    color: Vec3::broadcast(f32::NAN),
                    material: Some(self.material.clone()),
                });
            }
        }

        if discrim == 0. {
            return;
        }

        {
            let t2 = (-b + discrim.sqrt()) / a;
            if t2 >= ray.max_t {
                return;
            }
            let opos2 = ropos + rodir * t2;
            let wpos2 = ray.pos + ray.dir * t2;
            let wnorm2 = self.wotrans.transposed().transform_vec3(opos2).normalized();
            let uv2 = Self::uv(opos2);
            if (t2 > 0.) && self.cutout(wpos2, ray.dir, wnorm2, t2, opos2, uv2, depth) {
                ray.hits.push(ray::Hit {
                    opos: opos2,
                    wpos: wpos2,
                    wdir: ray.dir,
                    wnorm: wnorm2,
                    t: t2,
                    norm: wnorm2,
                    uv: uv2,
                    color: Vec3::broadcast(f32::NAN),
                    material: Some(self.material.clone()),
                });
            }
        }
    }

    fn intersect(&self, rays: &mut ray::Rayx4, depth: u16) {
        // Ray Object-Space Position & Direction
        let ropos = wider::transform_pointx4d(&self.wotrans, &rays.pos);
        let rodir = wider::transform_vecx4d(&self.wotrans, &rays.dir);

        let a = rodir.mag_sq();
        let b = ropos.dot(rodir);
        let c = ropos.mag_sq() - 1.;
        let discrim = b * b - a * c;

        if discrim.cmp_lt(f64x4::ZERO).all() {
            return;
        }

        let ropos = wider::cast_s(ropos);
        let rodir = wider::cast_s(rodir);

        let t1 = wider::cast_f32((-b - discrim.sqrt()) / a);
        let opos1 = ropos + rodir * t1;
        let wpos1 = rays.pos + rays.dir * t1;
        let uv1 = Self::uvx4(opos1);

        let m1 = t1.cmp_ge(f32x4::ZERO) & self.cutoutx4(wpos1, rays.dir, t1, opos1, uv1, depth);

        if m1.all() {
            let wnorm = wider::transform_vecx4(&self.wotrans.transposed(), opos1).normalized();
            rays.hits.push(ray::Hitx4 {
                opos: opos1,
                wpos: wpos1,
                wdir: rays.dir,
                wnorm,
                t: t1,
                norm: wnorm,
                uv: uv1,
                color: Vec3x4::broadcast(wider::f32x4_NAN),
                material: core::array::from_fn(|_| Some(self.material.clone())),
            });
            return;
        }

        let t2 = wider::cast_f32((-b + discrim.sqrt()) / a);
        let opos2 = ropos + rodir * t2;

        let m2 = wider::cast_f32(discrim.cmp_ne(f64x4::ZERO))
            & t2.cmp_ge(f32x4::ZERO)
            & self.cutoutx4(
                rays.pos + rays.dir * t2,
                rays.dir,
                t2,
                opos2,
                Self::uvx4(opos2),
                depth,
            );

        let t = m1.blend(t1, m2.blend(t2, wider::f32x4_NAN));
        let opos = ropos + rodir * t;
        let wnorm = wider::transform_vecx4(&self.wotrans.transposed(), opos).normalized();

        rays.hits.push(ray::Hitx4 {
            opos,
            wpos: rays.pos + rays.dir * t,
            wdir: rays.dir,
            wnorm,
            t,
            norm: wnorm,
            uv: Self::uvx4(opos),
            color: Vec3x4::broadcast(wider::f32x4_NAN),
            material: core::array::from_fn(|_| Some(self.material.clone())),
        })
    }
}

impl Sphere {
    #[inline]
    fn cutout(
        &self,
        wpos: Vec3,
        wdir: Vec3,
        wnorm: Vec3,
        t: f32,
        opos: Vec3,
        uv: (f32, f32),
        depth: u16,
    ) -> bool {
        let Some(cutout) = self.cutout else {
            return true;
        };
        let mut ctx = RtShadeContext::new(
            &self.cutoutparams,
            wpos,
            wdir,
            wnorm,
            t,
            opos,
            wnorm,
            uv,
            depth,
            RtMaterial::zeroed(), // todo: grab the actual material
        );
        (unsafe { cutout(&mut ctx) } != 0)
    }

    #[inline]
    fn cutoutx4(
        &self,
        wpos: Vec3x4,
        wdir: Vec3x4,
        t: f32x4,
        opos: Vec3x4,
        uv: (f32x4, f32x4),
        depth: u16,
    ) -> f32x4 {
        let Some(cutout) = self.cutout else {
            return !f32x4::ZERO;
        };

        let wpos: [Vec3; 4] = wpos.into();
        let wdir: [Vec3; 4] = wdir.into();
        let wnorm: [Vec3; 4] = wider::transform_vecx4(&self.wotrans.transposed(), opos)
            .normalized()
            .into();
        let t = t.as_array_ref();
        let opos: [Vec3; 4] = opos.into();
        let (u, v) = (uv.0.as_array_ref(), uv.1.as_array_ref());

        let mut out = [u32::MAX; 4];
        out.par_iter_mut().enumerate().for_each(|(i, out)| {
            let mut ctx = RtShadeContext::new(
                &self.cutoutparams,
                wpos[i],
                wdir[i],
                wnorm[i],
                t[i],
                opos[i],
                wnorm[i],
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

    #[inline]
    pub fn extract_avg_scale(&self) -> f32 {
        let inv3 = self.wotrans.inversed().truncate();
        (inv3.cols[0].mag() + inv3.cols[1].mag() + inv3.cols[2].mag()) / 3.
    }

    #[inline]
    pub fn uv(opos: Vec3) -> (f32, f32) {
        (
            (f32::atan2(opos.x, opos.z) / (2. * core::f32::consts::PI)).rem_euclid(1.),
            f32::acos(opos.y.clamp(-1., 1.)) / core::f32::consts::PI,
        )
    }

    #[inline]
    pub fn uvx4(opos: Vec3x4) -> (f32x4, f32x4) {
        let mut u = f32x4::atan2(opos.x, opos.z) / (2. * f32x4::PI);
        u = u - u.floor();
        let v = f32x4::acos(opos.y.fast_min(f32x4::ONE).fast_max(-f32x4::ONE)) / f32x4::PI;
        (u, v)
    }

    #[inline]
    pub(in crate::engine) fn create(
        wotrans: Mat4,
        cutout: RtCutoutFn,
        cutoutparams: Option<Arc<[u8]>>,
        material: Arc<Material>,
    ) -> Self {
        Self {
            wotrans,
            cutout,
            cutoutparams,
            material,
        }
    }
}
