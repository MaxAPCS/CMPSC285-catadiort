use std::sync::Arc;

use ultraviolet::{Mat4, Vec3, Vec3x4};
use wide::CmpGt;
use wide::{CmpLt, CmpNe, f32x4, f64x4};

use super::Primitive;
use super::shadecontext;
use crate::api::{RtCutoutFn, RtMaterial, RtShaderFn, WorldState};
use crate::engine::ray::{Rayx4, ShadowRay};
use crate::engine::util;
use crate::engine::world::MEDIUM_IOR;

#[derive(Clone)]
pub(crate) struct Sphere {
    /// World to Object Transform ([Sphere::owtrans] inverse)
    wotrans: Mat4,

    /// Object to World Transform ([Sphere::wotrans] inverse)
    pub owtrans: Mat4,

    /// Cutout Shader Reference
    cutout: RtCutoutFn,
    cutoutparams: Option<Arc<[u8]>>,

    /// Surface Shader Reference
    shader: RtShaderFn,
    shaderparams: Option<Arc<[u8]>>,

    /// Material Properties
    pub material: RtMaterial,
}

impl Primitive for Sphere {
    fn bounds(&self) -> (Vec3, Vec3) {
        let center = self.owtrans.transform_point3(Vec3::zero());

        let m = self.owtrans.as_array();
        let ex = m[0].abs() + m[4].abs() + m[8].abs();
        let ey = m[1].abs() + m[5].abs() + m[9].abs();
        let ez = m[2].abs() + m[6].abs() + m[10].abs();
        let half = Vec3::new(ex, ey, ez);

        (center - half, center + half)
    }

    fn reflect(&self, wpos: Vec3, wdir: Vec3) -> Vec3 {
        let opos = self.wotrans.transform_point3(wpos);
        let wnorm = self.wotrans.transposed().transform_vec3(opos).normalized();
        wdir.reflected(wnorm)
    }

    fn reflectx4(&self, wpos: Vec3x4, wdir: Vec3x4) -> Vec3x4 {
        let opos = util::transform_pointx4(&self.wotrans, &wpos);
        let wnorm = util::transform_vecx4(&self.wotrans.transposed(), opos).normalized();
        wdir.reflected(wnorm)
    }

    #[inline]
    fn reflects(&self) -> bool {
        self.material.reflects()
    }

    fn refract(&self, wpos: Vec3, wdir: Vec3) -> (f32, Vec3) {
        let opos = self.wotrans.transform_point3(wpos);
        let wnorm = self.wotrans.transposed().transform_vec3(opos).normalized();
        util::refracted(wdir, wnorm, self.material.ior / MEDIUM_IOR)
    }

    fn refractx4(&self, wpos: Vec3x4, wdir: Vec3x4) -> (f32x4, Vec3x4) {
        let opos = util::transform_pointx4(&self.wotrans, &wpos);
        let wnorm = util::transform_vecx4(&self.wotrans.transposed(), opos).normalized();
        util::refractedx4(wdir, wnorm, (self.material.ior / MEDIUM_IOR).into())
    }

    #[inline]
    fn refracts(&self) -> bool {
        self.material.refracts()
    }

    fn shade(&self, wpos: Vec3, wdir: Vec3, time: f32, depth: u16) -> (Vec3, RtMaterial) {
        let opos = self.wotrans.transform_point3(wpos);
        let mut wnorm = self.wotrans.transposed().transform_vec3(opos).normalized();
        if wdir.dot(wnorm) > 0. {
            wnorm = -wnorm;
        }

        let Some(shader) = self.shader else {
            return (wnorm, self.material);
        };

        let mut ctx = shadecontext(
            wpos,
            wdir,
            wnorm,
            opos,
            time,
            Self::uv(opos),
            depth,
            self.shaderparams(),
            self.material,
        );
        unsafe { shader(&mut ctx) };
        (ctx.N.into(), ctx.m)
    }

    fn shadex4(
        &self,
        wpos: Vec3x4,
        wdir: Vec3x4,
        t: f32x4,
        depth: u16,
    ) -> (Vec3x4, [RtMaterial; 4]) {
        let opos = util::transform_pointx4(&self.wotrans, &wpos);
        let mut wnorm = util::transform_vecx4(&self.wotrans.transposed(), opos).normalized();
        wnorm = Vec3x4::blend(wdir.dot(wnorm).cmp_gt(f32x4::ZERO), -wnorm, wnorm);

        let Some(shader) = self.shader else {
            return (wnorm, [self.material; 4]);
        };

        let (u, v) = Self::uvx4(opos);
        let params = self.shaderparams();

        let wpos: [Vec3; 4] = wpos.into();
        let wdir: [Vec3; 4] = wdir.into();
        let wnorm: [Vec3; 4] = wnorm.into();
        let opos: [Vec3; 4] = opos.into();

        let mut out_wnorm = [Vec3::broadcast(f32::NAN); 4];
        let mut out_materials = [self.material; 4];
        for i in 0..4 {
            let mut ctx = shadecontext(
                wpos[i],
                wdir[i],
                wnorm[i],
                opos[i],
                t.to_array()[i],
                (u.to_array()[i], v.to_array()[i]),
                depth,
                params,
                self.material,
            );
            unsafe { shader(&mut ctx) };
            out_wnorm[i] = ctx.N.into();
            out_materials[i] = ctx.m
        }
        (out_wnorm.into(), out_materials)
    }

    fn cutout(&self, wpos: Vec3, wdir: Vec3, opos: Vec3, time: f32, depth: u16) -> bool {
        let Some(cutout) = self.cutout else {
            return true;
        };

        let mut wnorm = self.wotrans.transposed().transform_vec3(opos).normalized();
        if wdir.dot(wnorm) > 0. {
            wnorm = -wnorm;
        }

        let mut ctx = shadecontext(
            wpos,
            wdir,
            wnorm,
            opos,
            time,
            Self::uv(opos),
            depth,
            self.cutoutparams(),
            self.material,
        );
        (unsafe { cutout(&mut ctx) } != 0)
    }

    #[inline]
    fn cutoutx4(&self, wpos: Vec3x4, wdir: Vec3x4, opos: Vec3x4, t: f32x4, depth: u16) -> f32x4 {
        let Some(cutout) = self.cutout else {
            return f32x4::ONE;
        };

        let mut wnorm = util::transform_vecx4(&self.wotrans.transposed(), opos).normalized();
        wnorm = Vec3x4::blend(wdir.dot(wnorm).cmp_gt(f32x4::ZERO), -wnorm, wnorm);

        let (u, v) = Self::uvx4(opos);
        let params = self.cutoutparams();

        let wpos: [Vec3; 4] = wpos.into();
        let wdir: [Vec3; 4] = wdir.into();
        let wnorm: [Vec3; 4] = wnorm.into();
        let opos: [Vec3; 4] = opos.into();

        let x: [_; 4] = core::array::from_fn(|i| {
            let time = t.to_array()[i];
            if time.is_nan() {
                return 0.;
            }
            let mut ctx = shadecontext(
                wpos[i],
                wdir[i],
                wnorm[i],
                opos[i],
                time,
                (u.to_array()[i], v.to_array()[i]),
                depth,
                params,
                self.material,
            );
            let cut = unsafe { cutout(&mut ctx) } == 0;
            if cut { 0. } else { 1. }
        });
        x.into()
    }

    fn shadow(&self, ray: &ShadowRay, max_t: f32, depth: u16) -> bool {
        let ropos = self.wotrans.transform_point3(ray.pos);
        let rodir = self.wotrans.transform_vec3(ray.dir);

        let a = rodir.mag_sq();
        let b = ropos.dot(rodir);
        let c = ropos.mag_sq() - 1.;
        let discrim = b * b - a * c;

        if discrim < 0. {
            return false;
        }

        let t1 = (-b - discrim.sqrt()) / a;

        if t1 > max_t {
            return false;
        }

        if (t1 >= 0.)
            & self.cutout(
                ray.pos + ray.dir * t1,
                ray.dir,
                ropos + rodir * t1,
                t1,
                depth,
            )
        {
            return true;
        }

        if discrim == 0. {
            return false;
        }

        let t2 = (-b + discrim.sqrt()) / a;

        (t2 >= 0. && t2 <= max_t)
            & self.cutout(
                ray.pos + ray.dir * t1,
                ray.dir,
                ropos + rodir * t2,
                t2,
                depth,
            )
    }

    fn intersectx4(&self, rays: &Rayx4, depth: u16) -> f32x4 {
        // Ray Object-Space Position & Direction
        let ropos = util::transform_pointx4d(&self.wotrans, &rays.pos);
        let rodir = util::transform_vecx4d(&self.wotrans, &rays.dir);

        let a = rodir.mag_sq();
        let b = ropos.dot(rodir);
        let c = ropos.mag_sq() - 1.;
        let discrim = b * b - a * c;

        if discrim.cmp_lt(f64x4::ZERO).all() {
            return util::f32x4_NAN;
        }

        let ropos = util::cast_s(ropos);
        let rodir = util::cast_s(rodir);
        let t1 = util::cast_f32((-b - discrim.sqrt()) / a);

        let m1 = t1.cmp_gt(f32x4::ZERO)
            & self.cutoutx4(
                rays.pos + rays.dir * t1,
                rays.dir,
                ropos + rodir * t1,
                t1,
                depth,
            );

        if m1.all() {
            return t1;
        }

        let t2 = util::cast_f32(
            discrim
                .cmp_ne(f64x4::ZERO)
                .blend((-b + discrim.sqrt()) / a, util::f64x4_NAN),
        );

        let m2 = self.cutoutx4(
            rays.pos + rays.dir * t2,
            rays.dir,
            ropos + rodir * t2,
            t2,
            depth,
        );

        m1.blend(t1, m2.blend(t2, util::f32x4_NAN))
    }
}

impl Sphere {
    #[inline]
    fn shaderparams(&self) -> *const core::ffi::c_void {
        self.shaderparams
            .as_ref()
            .map_or(core::ptr::null(), |p| p.as_ptr().cast())
    }

    #[inline]
    fn cutoutparams(&self) -> *const core::ffi::c_void {
        if let Some(p) = self.cutoutparams.as_ref() {
            p.as_ptr().cast()
        } else {
            core::ptr::null()
        }
    }

    #[inline]
    pub fn uv(opos: Vec3) -> (f32, f32) {
        (
            (f32::atan2(opos.x, opos.z) / (2. * core::f32::consts::PI)) % 1.,
            f32::acos(opos.y) / core::f32::consts::PI,
        )
    }

    #[inline]
    pub fn uvx4(opos: Vec3x4) -> (f32x4, f32x4) {
        let mut u = f32x4::atan2(opos.x, opos.z) / (2. * f32x4::PI);
        u = u - u.floor();
        let v = f32x4::acos(opos.y) / f32x4::PI;
        (u, v)
    }

    pub fn create(state: &WorldState, ctm: Mat4) -> Self {
        Self {
            wotrans: ctm.inversed(),
            owtrans: ctm,
            cutout: state.cutout,
            cutoutparams: state.cutoutparams.clone(),
            shader: state.shader,
            shaderparams: state.shaderparams.clone(),
            material: state.material,
        }
    }
}
