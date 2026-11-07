// use std::sync::Arc;

// use ultraviolet::{Mat4, Vec3, Vec3x4};
// use wide::{CmpGe, CmpGt, CmpLe};
// use wide::{CmpLt, CmpNe, f32x4, f64x4};

// use super::Primitive;
// use super::shadecontext;
// use crate::api::{RtCutoutFn, RtMaterial, RtShaderFn, WorldState};
// use crate::engine::ray::{Rayx4, ShadowRay};
// use crate::engine::util;
// use crate::engine::world::MEDIUM_IOR;

// pub(crate) struct Mesh<'a> {
//     wotrans: Mat4,

//     triangles: Vec<Tri<'a>>,

//     /// Cutout Shader Reference
//     cutout: RtCutoutFn,
//     cutoutparams: Option<Arc<[u8]>>,

//     /// Surface Shader Reference
//     shader: RtShaderFn,
//     shaderparams: Option<Arc<[u8]>>,

//     /// Material Properties
//     pub material: RtMaterial,
// }

// impl Primitive for Mesh<'_> {
//     fn intersectx4(&self, rays: &Rayx4, depth: u16) -> f32x4 {}
// }

// struct Tri<'a> {
//     wotrans: &'a Mat4,
//     vertices: [&'a Vec3; 3],
//     normals: Option<[&'a Vec3; 3]>,
//     colors: Option<[&'a Vec3; 3]>,
//     uvs: Option<[(f32, f32); 3]>,
// }

// impl Primitive for Tri<'_> {
//     fn reflect(&self, wpos: Vec3, wdir: Vec3) -> Vec3 {
//         let opos = self.wotrans.transform_point3(wpos);
//         let wnorm = self.wotrans.transposed().transform_vec3(opos).normalized();
//         wdir.reflected(wnorm)
//     }

//     fn reflectx4(&self, wpos: Vec3x4, wdir: Vec3x4) -> Vec3x4 {
//         let opos = util::transform_pointx4(&self.wotrans, &wpos);
//         let wnorm = util::transform_vecx4(&self.wotrans.transposed(), opos).normalized();
//         wdir.reflected(wnorm)
//     }

//     #[inline]
//     fn reflects(&self) -> bool {
//         self.material.reflects()
//     }

//     fn refract(&self, wpos: Vec3, wdir: Vec3) -> (f32, Vec3) {
//         let opos = self.wotrans.transform_point3(wpos);
//         let wnorm = self.wotrans.transposed().transform_vec3(opos).normalized();
//         util::refracted(wdir, wnorm, self.material.ior / MEDIUM_IOR)
//     }

//     fn refractx4(&self, wpos: Vec3x4, wdir: Vec3x4) -> (f32x4, Vec3x4) {
//         let opos = util::transform_pointx4(&self.wotrans, &wpos);
//         let wnorm = util::transform_vecx4(&self.wotrans.transposed(), opos).normalized();
//         util::refractedx4(wdir, wnorm, (self.material.ior / MEDIUM_IOR).into())
//     }

//     #[inline]
//     fn refracts(&self) -> bool {
//         self.material.refracts()
//     }

//     fn shade(&self, wpos: Vec3, wdir: Vec3, time: f32, depth: u16) -> (Vec3, RtMaterial) {
//         let opos = self.wotrans.transform_point3(wpos);

//         let mut wnorm = self.norm(bc);
//         if wdir.dot(wnorm) > 0. {
//             wnorm = -wnorm;
//         }

//         let Some(shader) = self.shader else {
//             return (wnorm, self.material);
//         };

//         let mut ctx = shadecontext(
//             wpos,
//             wdir,
//             wnorm,
//             opos,
//             time,
//             Self::uv(opos),
//             depth,
//             self.shaderparams(),
//             self.material,
//         );
//         unsafe { shader(&mut ctx) };
//         (ctx.N.into(), ctx.m)
//     }

//     fn shadex4(
//         &self,
//         wpos: Vec3x4,
//         wdir: Vec3x4,
//         t: f32x4,
//         depth: u16,
//     ) -> (Vec3x4, [RtMaterial; 4]) {
//         let opos = util::transform_pointx4(&self.wotrans, &wpos);
//         let mut wnorm = util::transform_vecx4(&self.wotrans.transposed(), opos).normalized();
//         wnorm = Vec3x4::blend(wdir.dot(wnorm).cmp_gt(f32x4::ZERO), -wnorm, wnorm);

//         let Some(shader) = self.shader else {
//             return (wnorm, [self.material; 4]);
//         };

//         let (u, v) = Self::uvx4(opos);
//         let params = self.shaderparams();

//         let wpos: [Vec3; 4] = wpos.into();
//         let wdir: [Vec3; 4] = wdir.into();
//         let wnorm: [Vec3; 4] = wnorm.into();
//         let opos: [Vec3; 4] = opos.into();

//         let mut out_wnorm = [Vec3::broadcast(f32::NAN); 4];
//         let mut out_materials = [self.material; 4];
//         for i in 0..4 {
//             let mut ctx = shadecontext(
//                 wpos[i],
//                 wdir[i],
//                 wnorm[i],
//                 opos[i],
//                 t.to_array()[i],
//                 (u.to_array()[i], v.to_array()[i]),
//                 depth,
//                 params,
//                 self.material,
//             );
//             unsafe { shader(&mut ctx) };
//             out_wnorm[i] = ctx.N.into();
//             out_materials[i] = ctx.m
//         }
//         (out_wnorm.into(), out_materials)
//     }

//     fn cutout(&self, wpos: Vec3, wdir: Vec3, opos: Vec3, time: f32, depth: u16) -> bool {
//         let Some(cutout) = self.cutout else {
//             return true;
//         };

//         let mut wnorm = self.wotrans.transposed().transform_vec3(opos).normalized();
//         if wdir.dot(wnorm) > 0. {
//             wnorm = -wnorm;
//         }

//         let mut ctx = shadecontext(
//             wpos,
//             wdir,
//             wnorm,
//             opos,
//             time,
//             Self::uv(opos),
//             depth,
//             self.cutoutparams(),
//             self.material,
//         );
//         (unsafe { cutout(&mut ctx) } != 0)
//     }

//     #[inline]
//     fn cutoutx4(&self, wpos: Vec3x4, wdir: Vec3x4, opos: Vec3x4, t: f32x4, depth: u16) -> f32x4 {
//         let Some(cutout) = self.cutout else {
//             return f32x4::ONE;
//         };

//         let mut wnorm = util::transform_vecx4(&self.wotrans.transposed(), opos).normalized();
//         wnorm = Vec3x4::blend(wdir.dot(wnorm).cmp_gt(f32x4::ZERO), -wnorm, wnorm);

//         let (u, v) = Self::uvx4(opos);
//         let params = self.cutoutparams();

//         let wpos: [Vec3; 4] = wpos.into();
//         let wdir: [Vec3; 4] = wdir.into();
//         let wnorm: [Vec3; 4] = wnorm.into();
//         let opos: [Vec3; 4] = opos.into();

//         let x: [_; 4] = core::array::from_fn(|i| {
//             let time = t.to_array()[i];
//             if time.is_nan() {
//                 return 0.;
//             }
//             let mut ctx = shadecontext(
//                 wpos[i],
//                 wdir[i],
//                 wnorm[i],
//                 opos[i],
//                 time,
//                 (u.to_array()[i], v.to_array()[i]),
//                 depth,
//                 params,
//                 self.material,
//             );
//             let cut = unsafe { cutout(&mut ctx) } == 0;
//             if cut { 0. } else { 1. }
//         });
//         x.into()
//     }

//     fn shadow(&self, ray: &ShadowRay, max_t: f32, depth: u16) -> bool {
//         let ropos = self.wotrans.transform_point3(ray.pos);
//         let rodir = self.wotrans.transform_vec3(ray.dir);

//         let a = rodir.mag_sq();
//         let b = ropos.dot(rodir);
//         let c = ropos.mag_sq() - 1.;
//         let discrim = b * b - a * c;

//         if discrim < 0. {
//             return false;
//         }

//         let t1 = (-b - discrim.sqrt()) / a;

//         if t1 > max_t {
//             return false;
//         }

//         if (t1 >= 0.)
//             & self.cutout(
//                 ray.pos + ray.dir * t1,
//                 ray.dir,
//                 ropos + rodir * t1,
//                 t1,
//                 depth,
//             )
//         {
//             return true;
//         }

//         if discrim == 0. {
//             return false;
//         }

//         let t2 = (-b + discrim.sqrt()) / a;

//         (t2 >= 0. && t2 <= max_t)
//             & self.cutout(
//                 ray.pos + ray.dir * t1,
//                 ray.dir,
//                 ropos + rodir * t2,
//                 t2,
//                 depth,
//             )
//     }

//     fn intersectx4(&self, rays: &Rayx4, depth: u16) -> f32x4 {
//         let e1 = Vec3x4::splat(*self.vertices[1] - *self.vertices[0]);
//         let e2 = Vec3x4::splat(*self.vertices[2] - *self.vertices[0]);

//         let ray_cross_e2 = rays.dir.cross(e2);
//         let det = e1.dot(ray_cross_e2);
//         let mut m = det.cmp_le(-f32::EPSILON) | det.cmp_gt(f32::EPSILON);
//         if m.none() {
//             return util::f32x4_NAN; // This ray is parallel to this triangle.
//         }

//         let inv_det = det.recip();
//         let s = rays.pos - Vec3x4::splat(*self.vertices[0]);
//         let u = inv_det * s.dot(ray_cross_e2);
//         m &= u.cmp_ge(0.) & u.cmp_le(1.);
//         if m.none() {
//             return util::f32x4_NAN;
//         }

//         let s_cross_e1 = s.cross(e1);
//         let v = inv_det * rays.dir.dot(s_cross_e1);
//         m &= v.cmp_ge(0.) & (u + v).cmp_le(1.);
//         if m.none() {
//             return util::f32x4_NAN;
//         }

//         let t = inv_det * e2.dot(s_cross_e1);

//         let ropos = util::transform_pointx4(self.wotrans, &rays.pos);
//         let rodir = util::transform_vecx4(self.wotrans, rays.dir);
//         m &= self.cutoutx4(
//             rays.pos + rays.dir * t,
//             rays.dir,
//             ropos + rodir * t,
//             t,
//             depth,
//         );

//         m.blend(t, util::f32x4_NAN)
//     }
// }

// impl Tri<'_> {
//     #[inline]
//     fn norm(&self, bc: (f32, f32)) -> Vec3 {
//         if let Some(norms) = self.normals {
//             trilerp(bc, norms.map(|n| *n))
//         } else {
//             let e1 = *self.vertices[1] - *self.vertices[0];
//             let e2 = *self.vertices[2] - *self.vertices[0];
//             Vec3::cross(&e1, e2)
//         }
//     }
// }

// #[inline]
// /// n.b. uses last two barycentric weights
// fn trilerp<T: Copy + std::ops::Sub<Output = T> + std::ops::Add<Output = T>>(
//     (b, c): (f32, f32),
//     values: [T; 3],
// ) -> T
// where
//     f32: std::ops::Mul<T, Output = T>,
// {
//     values[0] + b * (values[1] - values[0]) + c * (values[2] - values[0])
// }
