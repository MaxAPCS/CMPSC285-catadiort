use ultraviolet::Vec3;

use super::Light;
use super::ray::ShadowRay;

pub(crate) struct DirectionalLight {
    dir: Vec3,
    irradiance: Vec3,
}

impl Light for DirectionalLight {
    fn rayto(&self, pos: Vec3) -> ShadowRay {
        ShadowRay {
            pos,
            dir: self.dir,
            max_t: f32::INFINITY,
            irradiance: self.irradiance,
        }
    }
}

impl DirectionalLight {
    pub const fn new(dir: Vec3, irradiance: Vec3) -> Self {
        Self { dir, irradiance }
    }
}
