use ultraviolet::Vec3;

use super::Light;
use crate::engine::ray::ShadowRay;

pub struct DirectionalLight {
    pub dir: Vec3,
    pub irradiance: Vec3,
}

impl Light for DirectionalLight {
    #[inline]
    fn rayto(&self, pos: Vec3) -> ShadowRay {
        ShadowRay::new(pos, -self.dir, f32::INFINITY, self.irradiance)
    }
}
