use ultraviolet::Vec3;

use super::Light;
use super::ray::ShadowRay;

pub(crate) struct PlaneLight {
    pub(super) pos: Vec3,
    pub(super) emission: Vec3,
    pub(super) norm: Vec3,
}

impl Light for PlaneLight {
    fn rayto(&self, pos: Vec3) -> ShadowRay {
        let dir = self.pos - pos;
        let dst_sq = dir.mag_sq();
        let dst = dst_sq.sqrt();
        ShadowRay {
            pos,
            dir: dir / dst,
            max_t: dst,
            irradiance: self.emission * dir.dot(self.norm).max(0.),
        }
    }
}
