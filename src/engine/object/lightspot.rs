use ultraviolet::Vec3;

use super::Light;
use super::ray::ShadowRay;

pub(crate) struct SpotLight {
    pos: Vec3,
    dir: Vec3,
    intensity: Vec3,
    inner_cos: f32,
    outer_cos: f32,
    outer_dst: f32,
}

impl Light for SpotLight {
    fn rayto(&self, pos: Vec3) -> ShadowRay {
        let dir = pos - self.pos;
        let dst_sq = dir.mag_sq();
        let dst = dst_sq.sqrt();
        let dir = dir / dst;
        let c = dir.dot(self.dir);

        let cone = if c >= self.inner_cos {
            1.
        } else if c <= self.outer_cos {
            0.
        } else {
            let u = (c - self.outer_cos) / (self.inner_cos - self.outer_cos);
            u * u * (3. - 2. * u)
        };

        let reach = if self.outer_dst == 0. {
            1.
        } else if dst >= self.outer_dst {
            0.
        } else {
            let u = dst / self.outer_dst;
            (1. - u * u) * (1. - u * u)
        };

        ShadowRay {
            pos,
            dir: -dir,
            max_t: dst,
            irradiance: self.intensity * cone * reach / dst_sq,
        }
    }
}

impl SpotLight {
    pub fn new(
        pos: Vec3,
        dir: Vec3,
        intensity: Vec3,
        cone_angle: f32,
        penumbra_angle: f32,
        max_distance: f32,
    ) -> Self {
        Self {
            pos,
            dir,
            intensity,
            inner_cos: cone_angle.cos(),
            outer_cos: (cone_angle - penumbra_angle).cos(),
            outer_dst: max_distance,
        }
    }
}
