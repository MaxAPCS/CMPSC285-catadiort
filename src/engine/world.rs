#[cfg(feature = "bvh")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::sync::Arc;
use ultraviolet::{Mat4, Rotor3, Rotor3x4, Vec2, Vec3, Vec3x4, Vec4, Vec4x4};
use wide::f32x4;

#[cfg(feature = "bvh")]
use super::bvh::BVH;
use super::{Light, Material, Primitive, object, ray};
use crate::api::RtCutoutFn;
use crate::utils::{self, samplers, texture};

pub(crate) const MEDIUM_IOR: f32 = 1.0;

pub(crate) struct World {
    pub directionallights: Vec<object::DirectionalLight>,
    meshlights: Vec<Option<object::MeshEmissive>>,
    pub pointlights: Vec<object::PointLight>,
    spherelights: Vec<object::SphereLight>,
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

    pub materials: smallvec::SmallVec<[Material; 12]>,
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
            meshlights: Vec::new(),
            pointlights: Vec::new(),
            spherelights: Vec::new(),
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
                    [const { Material::default() }; _],
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
        let obj = object::Sphere::create(owtrans, cutout, cutoutparams, material);
        if let Some(sl) = object::SphereLight::from_emissive(&obj) {
            self.spherelights.push(sl);
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
        let prop = material.prop;
        let meshref = object::MeshReference {
            owtrans,
            cutout,
            cutoutparams,
            material,
            triref: self.meshtris.len(),
        };
        self.meshlights.push(object::from_emissive(
            &triangles,
            owtrans,
            &prop,
            object::MeshLight::emissive_create(&triangles, &meshref),
        ));
        self.meshinstances.push(meshref);
        self.meshtris.push((hash, triangles));
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
        let prop = material.prop;
        let meshref = object::MeshReference {
            owtrans,
            cutout,
            cutoutparams,
            material,
            triref,
        };
        self.meshlights.push(object::from_emissive(
            &self.meshtris[triref].1,
            owtrans,
            &prop,
            object::MeshLight::emissive_instance(
                self.meshlights[triref].as_ref().map(|l| &l.0),
                &meshref,
            ),
        ));
        self.meshinstances.push(meshref);
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
        self.meshlights = core::mem::take(&mut self.meshlights)
            .into_iter()
            .filter(|l| l.is_some())
            .collect();
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

    // whittedism
    pub(super) fn shadowrays(&self, point: Vec3) -> impl Iterator<Item = ray::ShadowRay> {
        let nanuv = Vec2::broadcast(f32::NAN);
        core::iter::chain(
            core::iter::chain(
                self.pointlights.iter().map(move |l| l.rayto(point, nanuv)),
                core::iter::chain(
                    self.meshlights.iter().map(move |l| {
                        l.as_ref()
                            .expect("World::shadowrays called before World::finish")
                            .1
                            .rayto(point, nanuv)
                    }),
                    self.spherelights.iter().map(move |l| l.rayto(point, nanuv)),
                ),
            ),
            core::iter::chain(
                self.directionallights
                    .iter()
                    .map(move |l| l.rayto(point, nanuv)),
                self.spotlights.iter().map(move |l| l.rayto(point, nanuv)),
            ),
        )
    }

    pub(super) fn lightrays(
        &self,
        point: &Vec3,
        uv: &mut samplers::AnySampler,
        samples: u16,
    ) -> Vec<(ray::ShadowRay, f32)> {
        let mut out = Vec::with_capacity(
            self.pointlights.len()
                + self.meshlights.len() * samples as usize
                + self.directionallights.len()
                + self.spotlights.len(),
        );
        let n = utils::factor(samples);
        let m = samples / n;
        let invsamp = (samples as f32).recip();
        for l in self.pointlights.iter() {
            out.push((l.rayto(*point, uv.next2()), l.cos_angular_radius(*point)));
        }
        for l in self.meshlights.iter() {
            let l = &l
                .as_ref()
                .expect("World::lightrays called before World::finish")
                .0;
            let c = l.cos_angular_radius(*point);
            let sample = uv.next2();
            for i in 0..m {
                for j in 0..n {
                    let uv = Vec2::new(
                        ((i as f32 + sample.x) / m as f32).min(1.),
                        ((j as f32 + sample.y) / n as f32).min(1.),
                    );
                    let mut r = l.rayto(*point, uv);
                    r.irradiance *= invsamp;
                    out.push((r, c));
                }
            }
        }
        for l in self.directionallights.iter() {
            out.push((l.rayto(*point, uv.next2()), l.cos_angular_radius(*point)));
        }
        for l in self.spherelights.iter() {
            let c = l.cos_angular_radius(*point);
            let sample = uv.next2();
            for i in 0..m {
                for j in 0..n {
                    let uv = Vec2::new(
                        ((i as f32 + sample.x) / m as f32).min(1.),
                        ((j as f32 + sample.y) / n as f32).min(1.),
                    );
                    let mut r = l.rayto(*point, uv);
                    r.irradiance *= invsamp;
                    out.push((r, c));
                }
            }
        }
        for l in self.spotlights.iter() {
            out.push((l.rayto(*point, uv.next2()), l.cos_angular_radius(*point)));
        }
        out
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
