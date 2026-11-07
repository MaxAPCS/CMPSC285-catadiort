use ultraviolet::Vec3;

use crate::engine::{Sphere, ray::ShadowRay, util};

use super::Light;

pub(crate) struct PointLight {
    pos: Vec3,
    intensity: Vec3,
}

impl Light for PointLight {
    fn rayto(&self, pos: Vec3) -> ShadowRay {
        let dir = self.pos - pos;
        let dst_sq = dir.mag_sq();
        let dst = dst_sq.sqrt();
        ShadowRay {
            pos,
            dir: dir / dst,
            max_t: dst,
            irradiance: Some(self.intensity / dst_sq),
        }
    }
}

impl PointLight {
    pub const fn new(pos: Vec3, intensity: Vec3) -> Self {
        Self { pos, intensity }
    }

    pub fn from_emissive(o: &Sphere) -> Option<Self> {
        if o.material.emission == Vec3::zero() {
            return None;
        }

        let intensity = util::extract_scale_sq(&o.owtrans)
            .as_array()
            .iter()
            .sum::<f32>()
            / 9.
            * core::f32::consts::PI
            * o.material.emission;

        Some(Self {
            pos: o.owtrans.extract_translation(),
            intensity,
        })
    }
}
