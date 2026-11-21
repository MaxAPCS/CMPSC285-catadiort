use ultraviolet::{Vec2, Vec3};

use super::Light;
use crate::engine::ray::ShadowRay;

pub struct PointLight {
    pub pos: Vec3,
    pub intensity: Vec3,
    pub(super) radius: f32,
}

impl Light for PointLight {
    fn rayto(&self, pos: Vec3, _: Vec2) -> ShadowRay {
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
}
