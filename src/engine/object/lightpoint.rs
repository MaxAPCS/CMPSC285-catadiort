use ultraviolet::Vec3;

use super::{Light, Primitive, Sphere};
use crate::{api::RtMaterial, engine::ray::ShadowRay};

pub struct PointLight {
    pub pos: Vec3,
    pub intensity: Vec3,
    radius: f32,
}

impl Light for PointLight {
    fn rayto(&self, pos: Vec3) -> ShadowRay {
        let dir = self.pos - pos;
        let dst_sq = dir.mag_sq();
        let dst = dst_sq.sqrt();
        ShadowRay::new(pos, dir / dst, dst - self.radius, self.intensity / dst_sq)
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

        let scale = o.extract_avg_scale();
        let intensity = scale * scale * core::f32::consts::PI * material.emission;

        Some(Self {
            pos: o.centroid(),
            intensity,
            radius: scale.sqrt(),
        })
    }
}
