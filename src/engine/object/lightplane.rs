use ultraviolet::Vec3;

use super::Light;
use super::ray::ShadowRay;
use crate::engine::RECAST_EPSILON;

pub(crate) struct PlaneLight {
    pub(super) pos: Vec3,
    pub(super) intensity: Vec3,
    pub(super) norm: Vec3,
}

impl Light for PlaneLight {
    fn rayto(&self, pos: Vec3) -> ShadowRay {
        let dir = (self.pos + self.norm * RECAST_EPSILON) - pos;
        let dst = dir.mag();
        let realdir = pos - self.pos;
        let realdist_sq = realdir.mag_sq();
        let realdir = realdir / realdist_sq.sqrt();
        ShadowRay {
            pos,
            dir: dir / dst,
            max_t: dst,
            irradiance: self.intensity * realdir.dot(self.norm).max(0.) / realdist_sq,
        }
    }
}
