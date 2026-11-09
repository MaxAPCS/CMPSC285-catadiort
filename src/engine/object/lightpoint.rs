use ultraviolet::Vec3;

use super::Light;
use super::Primitive;
use super::{Sphere, ray::ShadowRay};
use crate::api::RtMaterial;

pub(crate) struct PointLight {
    pos: Vec3,
    intensity: Vec3,

    radius: f32,
}

impl Light for PointLight {
    fn rayto(&self, pos: Vec3) -> ShadowRay {
        let dir = self.pos - pos;
        let dst_sq = dir.mag_sq();
        let dst = dst_sq.sqrt();
        ShadowRay {
            pos,
            dir: dir / dst,
            max_t: dst - self.radius,
            irradiance: self.intensity / dst_sq,
        }
    }
}

impl PointLight {
    pub const fn new(pos: Vec3, intensity: Vec3) -> Self {
        Self {
            pos,
            intensity,
            radius: 0.,
        }
    }

    pub fn from_emissive(o: &Sphere, material: &RtMaterial) -> Option<Self> {
        if material.emission == Vec3::zero() {
            return None;
        }

        let scale_sq = o.extract_avg_scale_sq();
        let intensity = scale_sq * core::f32::consts::PI * material.emission;

        Some(Self {
            pos: o.centroid(),
            intensity,
            radius: scale_sq.sqrt(),
        })
    }
}
