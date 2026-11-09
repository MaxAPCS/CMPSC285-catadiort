use ultraviolet::{Rotor3, Rotor3x4, Vec3, Vec3x4, Vec4, Vec4x4};
use wide::f32x4;

use super::object;
use super::ray;
use super::{Light, Material};
use crate::engine::Primitive;
use crate::utils::texture;

pub const MEDIUM_IOR: f32 = 1.0;

pub(crate) struct World {
    pub materials: smallvec::SmallVec<[Material; 4]>,
    pub spheres: Vec<object::Sphere>,
    pub meshes: Vec<object::Mesh>,
    pub pointlights: Vec<object::PointLight>,
    pub planelights: Vec<object::PlaneLight>,
    pub directionallights: Vec<object::DirectionalLight>,
    pub spotlights: Vec<object::SpotLight>,

    pub ambientcolor: Vec3,
    pub environmentcolor: Vec3,
    pub environmenttex: i32,
    pub environmentscale: f32,
    pub environmentrotation: f32,
}

impl World {
    #[inline]
    pub const fn empty() -> Self {
        Self {
            materials: unsafe {
                smallvec::SmallVec::from_const_with_len_unchecked(
                    [const { Material::default() }; 4],
                    1,
                )
            },
            spheres: Vec::new(),
            meshes: Vec::new(),
            pointlights: Vec::new(),
            planelights: Vec::new(),
            directionallights: Vec::new(),
            spotlights: Vec::new(),

            ambientcolor: Vec3::broadcast(0.),
            environmentcolor: Vec3::broadcast(0.),
            environmenttex: -1,
            environmentscale: 1.,
            environmentrotation: 0.,
        }
    }

    /// Build acceleration structures
    pub fn finish(&mut self) {
        // pass
    }

    pub(super) fn intersect(&self, rays: &mut ray::Rayx4, depth: u16) {
        for s in &self.spheres {
            rays.intersect(s, depth)
        }
        for m in &self.meshes {
            rays.intersect(m, depth);
        }
    }

    pub(super) fn shadow(&self, rays: &mut ray::ShadowRay, depth: u16) {
        for s in &self.spheres {
            if rays.intersect(s, depth) {
                return;
            }
        }
        for m in &self.meshes {
            if rays.intersect(m, depth) {
                return;
            }
        }
    }

    pub(super) fn shadowrays(&self, point: Vec3) -> impl Iterator<Item = ray::ShadowRay> {
        core::iter::chain(
            core::iter::chain(
                self.pointlights.iter().map(move |l| l.rayto(point)),
                self.planelights.iter().map(move |l| l.rayto(point)),
            ),
            core::iter::chain(
                self.directionallights.iter().map(move |l| l.rayto(point)),
                self.spotlights.iter().map(move |l| l.rayto(point)),
            ),
        )
    }

    pub(super) fn sample_environment(&self, dir: Vec3) -> Vec3 {
        if dir.x.is_nan() {
            return Vec3::zero();
        }
        if self.environmenttex < 0 {
            return self.environmentcolor;
        }
        let (u, v) =
            object::Sphere::uv(dir.rotated_by(Rotor3::from_rotation_xz(self.environmentrotation)));
        let tex = texture::lookup_lat_long(self.environmenttex, u, v) * self.environmentscale;
        Vec3::from_homogeneous_point(tex) + self.environmentcolor * (1. - tex.w)
    }

    pub(super) fn sample_environmentx4(&self, dir: Vec3x4) -> Vec3x4 {
        if dir.x.is_nan().all() {
            return Vec3x4::zero();
        }
        if self.environmenttex < 0 {
            return Vec3x4::splat(self.environmentcolor);
        }
        let (u, v) = object::Sphere::uvx4(
            dir.rotated_by(Rotor3x4::from_rotation_xz(self.environmentrotation.into())),
        );
        let (u, v): ([f32; 4], [f32; 4]) = (u.into(), v.into());
        let tex: [Vec4; 4] =
            core::array::from_fn(|i| texture::lookup_lat_long(self.environmenttex, u[i], v[i]));
        let tex: Vec4x4 = tex.into();
        Vec3x4::blend(
            dir.x.is_nan(),
            Vec3x4::zero(),
            Vec3x4::from_homogeneous_point(tex * f32x4::splat(self.environmentscale))
                + Vec3x4::splat(self.environmentcolor) * (f32x4::ONE - tex.w),
        )
    }
}

impl Default for World {
    fn default() -> Self {
        Self::empty()
    }
}
