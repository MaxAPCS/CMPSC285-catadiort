mod object;
mod ray;
mod screen;
mod util;
mod world;

pub(crate) use object::*;
pub(crate) use screen::*;
pub(crate) use world::*;

const RECAST_EPSILON: f32 = 10e-4;
const RECAST_EPSILON_X4: wide::f32x4 = wide::f32x4::new([RECAST_EPSILON; _]);
const DEPTH_COMPLEXITY: usize = 4;

impl crate::api::RtMaterial {
    #[inline]
    const fn reflects(&self) -> bool {
        const SQRT_0D15: f32 = 0.3872983346207416885179265399782399610832921705291590826587;

        self.transmission == 0.
            && self.roughness < SQRT_0D15
            && (self.specular.x > 0. || self.specular.y > 0. || self.specular.z > 0.)
    }

    #[inline]
    const fn refracts(&self) -> bool {
        self.transmission > 0.
    }

    #[inline]
    const fn smoothness(&self) -> f32 {
        (1. - self.roughness * self.roughness * 0.15f32.recip()).max(0.)
    }

    #[inline]
    const fn phongexponent(&self) -> i32 {
        let alpha = (self.roughness * self.roughness).clamp(0.05, 1.);
        (2. / (alpha * alpha) - 2.).max(1.) as _
    }

    #[inline]
    const fn fresnel(&self, dir_dot_norm: f32) -> f32 {
        if self.ior == world::MEDIUM_IOR {
            return 0.;
        }
        let f0 = (self.ior - world::MEDIUM_IOR) / (self.ior + world::MEDIUM_IOR);
        let f0 = f0 * f0;
        let t = 1. - dir_dot_norm;
        let t2 = t * t;
        f0 + (1. - f0) * (t2 * t2 * t)
    }

    #[inline]
    fn fresnelx4(&self, dir_dot_norm: wide::f32x4) -> wide::f32x4 {
        if self.ior == world::MEDIUM_IOR {
            return wide::f32x4::ZERO;
        }
        let f0 = (self.ior - world::MEDIUM_IOR) / (self.ior + world::MEDIUM_IOR);
        let f0 = f0 * f0;
        let t = 1. - dir_dot_norm;
        let t2 = t * t;
        f0 + (1. - f0) * (t2 * t2 * t)
    }

    /// Arguments:
    /// - obj_wnorm: Object's Worldspace Normal, normalized
    /// - ray_wdir: Ray's Worldspace Direction, normalized
    fn color(
        &self,
        shadowrays: &[ray::ShadowRay],
        obj_wnorm: ultraviolet::Vec3,
        ray_wdir: ultraviolet::Vec3,
        entering: bool,
        ambient: ultraviolet::Vec3,
    ) -> ultraviolet::Vec3 {
        let mut diffuse = ultraviolet::Vec3::zero();
        let mut specular = ultraviolet::Vec3::zero();
        for shadowray in shadowrays {
            if let Some(irradiance) = shadowray.irradiance {
                diffuse += irradiance * obj_wnorm.dot(shadowray.dir).max(0.);
                specular += irradiance
                    * shadowray
                        .dir
                        .reflected(obj_wnorm)
                        .dot(-ray_wdir)
                        .max(0.)
                        .powi(self.phongexponent())
            }
        }
        let external =
            (1. - self.transmission) * self.albedo * (ambient + diffuse / core::f32::consts::PI)
                + self.specular * specular;
        if entering {
            external + self.emission
        } else {
            external
        }
    }
}
