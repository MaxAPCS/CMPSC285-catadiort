use ultraviolet::{Vec2, Vec3};

use super::Light;
use crate::engine::ray::ShadowRay;

pub struct PlaneLight {
    pub pos: Vec3,
    pub intensity: Vec3,
    pub norm: Vec3,
}

impl Light for PlaneLight {
    fn rayto(&self, pos: Vec3, _: Vec2) -> ShadowRay {
        let dir = self.pos - pos;
        let dst_sq = dir.mag_sq();
        let dst = dst_sq.sqrt();
        let dir = dir / dst;
        ShadowRay::new(
            pos,
            dir,
            dst,
            self.intensity * dir.dot(-self.norm).max(0.) / dst_sq,
        )
    }
}
