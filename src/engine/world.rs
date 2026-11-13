use std::sync::Arc;

use ultraviolet::Mat4;
use ultraviolet::{Rotor3, Rotor3x4, Vec3, Vec3x4, Vec4, Vec4x4};
use wide::f32x4;

use super::object;
use super::ray;
use super::{Light, Material, Primitive};
use crate::api::{RtCutoutFn, RtMaterial};
#[cfg(feature = "bvh")]
use crate::engine::bvh::BVH;
use crate::engine::{MeshEmissive, from_emissive};
use crate::utils::texture;

pub const MEDIUM_IOR: f32 = 1.0;

pub(crate) struct World {
    pub directionallights: Vec<object::DirectionalLight>,
    planelights: Vec<object::PlaneLight>,
    pub pointlights: Vec<object::PointLight>,
    pub spotlights: Vec<object::SpotLight>,
    meshes: Vec<object::Mesh>,
    pub spheres: Vec<object::Sphere>,

    meshtris: Vec<(u64, Box<[object::Tri]>)>,
    meshinstances: Vec<object::MeshReference>,

    pub materials: smallvec::SmallVec<[Material; 4]>,
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
            directionallights: Vec::new(),
            planelights: Vec::new(),
            pointlights: Vec::new(),
            spotlights: Vec::new(),
            meshes: Vec::new(),
            spheres: Vec::new(),

            meshtris: Vec::new(),
            meshinstances: Vec::new(),

            materials: unsafe {
                smallvec::SmallVec::from_const_with_len_unchecked(
                    [const { Material::default() }; 4],
                    1,
                )
            },
            ambientcolor: Vec3::broadcast(0.),
            environmentcolor: Vec3::broadcast(0.),
            environmenttex: -1,
            environmentscale: 1.,
            environmentrotation: 0.,
        }
    }

    pub fn mesh_create(
        &mut self,
        hash: u64,
        owtrans: Mat4,
        cutout: RtCutoutFn,
        cutoutparams: Option<Arc<[u8]>>,
        material: u32,
        materialobj: RtMaterial,
        tris: Box<[object::Tri]>,
    ) {
        match from_emissive(&tris, owtrans.inversed().transposed(), &materialobj) {
            Some(MeshEmissive::Plane(pl)) => self.planelights.push(pl),
            Some(MeshEmissive::Point(pl)) => self.pointlights.push(pl),
            None => {}
        }
        self.meshtris.push((hash, tris));
        self.meshinstances.push(object::MeshReference {
            owtrans,
            cutout,
            cutoutparams,
            material,
            triref: self.meshtris.len() - 1,
        });
    }

    pub fn mesh_instance(
        &mut self,
        hash: u64,
        owtrans: Mat4,
        cutout: RtCutoutFn,
        cutoutparams: &Option<Arc<[u8]>>,
        material: u32,
        materialobj: RtMaterial,
    ) -> bool {
        let Some(triref) = self.meshtris.iter().position(|mt| mt.0 == hash) else {
            return false;
        };
        match from_emissive(
            &self.meshtris[triref].1,
            owtrans.inversed().transposed(),
            &materialobj,
        ) {
            Some(MeshEmissive::Plane(pl)) => self.planelights.push(pl),
            Some(MeshEmissive::Point(pl)) => self.pointlights.push(pl),
            None => {}
        }
        self.meshinstances.push(object::MeshReference {
            owtrans,
            cutout,
            cutoutparams: cutoutparams.clone(),
            material,
            triref,
        });
        true
    }

    /// Build acceleration structures, emissive standins
    pub fn finish(&mut self) {
        #[cfg(feature = "bvh")]
        let bvhs: Vec<_> = self
            .meshtris
            .drain(..)
            .map(|(_, m)| Arc::new(BVH::create(m)))
            .collect();
        #[cfg(not(feature = "bvh"))]
        let tris: Vec<Arc<[_]>> = self.meshtris.drain(..).map(|(_, m)| Arc::from(m)).collect();
        self.meshes = self
            .meshinstances
            .drain(..)
            .map(|meshref| {
                let triref = meshref.triref;
                object::Mesh::create(
                    meshref,
                    #[cfg(not(feature = "bvh"))]
                    tris[triref].clone(),
                    #[cfg(feature = "bvh")]
                    bvhs[triref].clone(),
                )
            })
            .collect();
    }

    pub(super) fn intersect(&self, rays: &mut ray::Rayx4, depth: u16) {
        for s in &self.spheres {
            s.intersect(rays, depth);
        }
        for m in &self.meshes {
            m.intersect(rays, depth);
        }
    }

    /// Returns:
    /// - occluded: Whether this ray is occluded
    pub(super) fn shadow(&self, ray: &mut ray::ShadowRay, depth: u16) -> bool {
        for s in &self.spheres {
            if ray.intersect(s, depth) {
                return true;
            }
        }
        for m in &self.meshes {
            if ray.intersect(m, depth) {
                return true;
            }
        }
        false
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
