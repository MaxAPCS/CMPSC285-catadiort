#[cfg(feature = "bvh")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::sync::Arc;
use ultraviolet::{Mat4, Rotor3, Rotor3x4, Vec3, Vec3x4, Vec4, Vec4x4};
use wide::f32x4;

#[cfg(feature = "bvh")]
use super::bvh::BVH;
use super::{
    Light, Material, MeshEmissive, PointLight, Primitive, Sphere, from_emissive, object, ray,
};
use crate::api::RtCutoutFn;
use crate::utils::texture;

pub(crate) const MEDIUM_IOR: f32 = 1.0;

pub(crate) struct World {
    pub directionallights: Vec<object::DirectionalLight>,
    planelights: Vec<object::PlaneLight>,
    pub pointlights: Vec<object::PointLight>,
    pub spotlights: Vec<object::SpotLight>,

    #[cfg(not(feature = "bvh"))]
    meshes: Vec<object::Mesh>,
    #[cfg(feature = "bvh")]
    meshbvh: Option<BVH<object::Mesh>>,

    meshtris: Vec<(u64, Box<[object::Tri]>)>,
    meshinstances: Vec<object::MeshReference>,

    pub spheres: Vec<object::Sphere>,
    #[cfg(feature = "bvh")]
    spherebvh: Option<BVH<object::Sphere>>,

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

            #[cfg(not(feature = "bvh"))]
            meshes: Vec::new(),
            #[cfg(feature = "bvh")]
            meshbvh: None,

            meshtris: Vec::new(),
            meshinstances: Vec::new(),

            spheres: Vec::new(),
            #[cfg(feature = "bvh")]
            spherebvh: None,

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

    pub fn sphere_create(
        &mut self,
        owtrans: Mat4,
        cutout: RtCutoutFn,
        cutoutparams: Option<Arc<[u8]>>,
        material: Arc<Material>,
    ) {
        let prop = material.prop;
        let obj = Sphere::create(owtrans, cutout, cutoutparams, material);
        if let Some(pl) = PointLight::from_emissive(&obj, &prop) {
            self.pointlights.push(pl);
        }
        self.spheres.push(obj);
    }

    pub fn mesh_create(
        &mut self,
        hash: u64,
        owtrans: Mat4,
        cutout: RtCutoutFn,
        cutoutparams: Option<Arc<[u8]>>,
        material: Arc<Material>,
        triangles: Box<[object::Tri]>,
    ) {
        match from_emissive(&triangles, owtrans, &material.prop) {
            Some(MeshEmissive::Plane(pl)) => self.planelights.push(pl),
            Some(MeshEmissive::Point(pl)) => self.pointlights.push(pl),
            None => {}
        }
        self.meshtris.push((hash, triangles));
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
        cutoutparams: Option<Arc<[u8]>>,
        material: Arc<Material>,
    ) -> bool {
        let Some(triref) = self.meshtris.iter().position(|mt| mt.0 == hash) else {
            return false;
        };
        match from_emissive(&self.meshtris[triref].1, owtrans, &material.prop) {
            Some(MeshEmissive::Plane(pl)) => self.planelights.push(pl),
            Some(MeshEmissive::Point(pl)) => self.pointlights.push(pl),
            None => {}
        }
        self.meshinstances.push(object::MeshReference {
            owtrans,
            cutout,
            cutoutparams,
            material,
            triref,
        });
        true
    }

    /// Build acceleration structures, emissive standins
    pub fn finish(&mut self) {
        #[cfg(not(feature = "bvh"))]
        let tris: Vec<Arc<[_]>> = self.meshtris.drain(..).map(|(_, m)| Arc::from(m)).collect();
        #[cfg(feature = "bvh")]
        let bvhs: Vec<_> = core::mem::take(&mut self.meshtris)
            .into_par_iter()
            .map(|(_, m)| Arc::new(BVH::create(m)))
            .collect();
        let meshes = self
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
        #[cfg(not(feature = "bvh"))]
        {
            self.meshes = meshes;
        }
        #[cfg(feature = "bvh")]
        {
            self.meshbvh = Some(BVH::create(meshes));
            self.spherebvh = Some(BVH::create(
                core::mem::take(&mut self.spheres).into_boxed_slice(),
            ))
        }
    }

    pub(super) fn intersect(&self, ray: &mut ray::Rayx4, depth: u16) {
        #[cfg(not(feature = "bvh"))]
        {
            for s in &self.spheres {
                s.intersect(ray, depth);
            }
            for m in &self.meshes {
                m.intersect(ray, depth);
            }
        }
        #[cfg(feature = "bvh")]
        {
            self.spherebvh
                .as_ref()
                .expect("World::intersect called before World::finish")
                .intersect(ray, depth);
            self.meshbvh
                .as_ref()
                .expect("World::intersect called before World::finish")
                .intersect(ray, depth);
        }
    }

    pub(super) fn obstruct(&self, ray: &mut ray::ShadowRay, depth: u16) {
        #[cfg(not(feature = "bvh"))]
        {
            for s in &self.spheres {
                s.obstruct(ray, depth);
            }
            for m in &self.meshes {
                m.obstruct(ray, depth);
            }
        }
        #[cfg(feature = "bvh")]
        {
            self.spherebvh
                .as_ref()
                .expect("World::obstruct called before World::finish")
                .obstruct(ray, depth);
            self.meshbvh
                .as_ref()
                .expect("World::obstruct called before World::finish")
                .obstruct(ray, depth);
        }
        ray.collapse();
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
