use core::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use ultraviolet::{Mat4, Vec3};

use super::*;
use crate::engine::{DirectionalLight, Material, Mesh, PointLight, SpotLight, World};

struct WorldState {
    pub shader: RtShaderFn,
    pub shaderparams: Option<Arc<[u8]>>,
    pub cutout: RtCutoutFn,
    pub cutoutparams: Option<Arc<[u8]>>,

    world: World,
}

impl WorldState {
    const fn new() -> Self {
        Self {
            shader: None,
            shaderparams: None,
            cutout: None,
            cutoutparams: None,

            world: World::empty(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    fn set_material(&mut self, material: Option<RtMaterial>) {
        let Some(material) = material else {
            self.world.materials.push(Material::default());
            return;
        };
        self.world.materials.push(Material::new(
            self.shader,
            self.shaderparams.clone(),
            material,
        ));
    }

    fn rebind_material(&mut self) {
        self.world.materials.push(Material::new(
            self.shader.clone(),
            self.shaderparams.clone(),
            self.last_material().prop,
        ));
    }

    fn last_material(&self) -> &Material {
        &self.world.materials[self.world.materials.len() - 1]
    }
}

static OBJECTSTATE: Mutex<WorldState> = Mutex::new(WorldState::new());

#[inline]
pub(super) fn take() -> World {
    core::mem::take(&mut OBJECTSTATE.lock().unwrap().world)
}

#[inline]
pub(super) fn clear() {
    OBJECTSTATE.lock().unwrap().reset();
}

/* attributes ------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub extern "C" fn rtMaterial(m: *const RtMaterial) {
    OBJECTSTATE.lock().unwrap().set_material(if m.is_null() {
        None
    } else {
        // Safety: `m` is null or points at an RtMaterial.
        Some(unsafe { *m })
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn rtSurface(f: RtShaderFn, params: *const core::ffi::c_void, nbytes: usize) {
    let mut os = OBJECTSTATE.lock().unwrap();
    os.shader = f;
    os.shaderparams = if params.is_null() {
        None
    } else {
        // Safety: `params` is null or points at `nbytes` bytes.
        Some(unsafe { core::slice::from_raw_parts::<u8>(params.cast(), nbytes) }.into())
    };
    os.rebind_material()
}

#[unsafe(no_mangle)]
pub extern "C" fn rtCutout(f: RtCutoutFn, params: *const core::ffi::c_void, nbytes: usize) {
    let mut os = OBJECTSTATE.lock().unwrap();
    os.cutout = f;
    os.cutoutparams = if params.is_null() {
        None
    } else {
        // Safety: `params` is null or points at `nbytes` bytes.
        Some(unsafe { core::slice::from_raw_parts::<u8>(params.cast(), nbytes) }.into())
    }
}

/* lights ----------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub extern "C" fn rtPointLight(position: &Vec3, intensity: &RtColor) {
    let pos = transform::get().transform_point3(*position);

    let obj = PointLight::new(pos, *intensity);
    OBJECTSTATE.lock().unwrap().world.pointlights.push(obj);
}

#[unsafe(no_mangle)]
pub extern "C" fn rtDirectionalLight(direction: &Vec3, &irradiance: &RtColor) {
    let dir = transform::get().transform_vec3(*direction).normalized();

    let obj = DirectionalLight { dir, irradiance };
    OBJECTSTATE
        .lock()
        .unwrap()
        .world
        .directionallights
        .push(obj);
}

/// The cone is stored as cosines; the penumbra is clamped into the cone.
#[unsafe(no_mangle)]
pub extern "C" fn rtSpotLight(
    position: &Vec3,
    direction: &Vec3,
    intensity: &RtColor,
    cone_angle: RtFloat,
    penumbra_angle: RtFloat,
    max_distance: RtFloat,
) {
    let pos = transform::get().transform_point3(*position);
    let dir = transform::get().transform_vec3(*direction);

    let obj = SpotLight::new(
        pos,
        dir,
        *intensity,
        cone_angle.to_radians(),
        penumbra_angle.to_radians(),
        max_distance,
    );
    OBJECTSTATE.lock().unwrap().world.spotlights.push(obj);
}

/* primitives ------------------------------------------------------------- */

/// A sphere of the given radius at the CTM's origin.
#[unsafe(no_mangle)]
pub extern "C" fn rtSphere(radius: RtFloat) {
    if radius <= 0. {
        #[cfg(debug_assertions)]
        println!("rtobject: skipped rtSphere with radius {}", radius);
        return;
    }
    let ctm = transform::get();
    let mut os = OBJECTSTATE.lock().unwrap();
    let cutout = os.cutout;
    let cutoutparams = os.cutoutparams.clone();
    let material = Arc::new(os.last_material().clone());

    os.world.sphere_create(
        ctm * Mat4::from_scale(radius),
        cutout,
        cutoutparams,
        material,
    );
}

/// # Safety
/// The arrays are as long as nverts and 3 ntris say; N, st and Cs may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rtTriangleMesh(
    nverts: RtInt,
    p: *const RtPoint,
    n: *const RtNormal,
    st: *const [RtFloat; 2],
    cs: *const RtColor,
    ntris: RtInt,
    indexes: *const [RtInt; 3],
) {
    if nverts <= 0 || ntris <= 0 || p.is_null() || indexes.is_null() {
        #[cfg(debug_assertions)]
        println!("rtobject: skipped rtTriangleMesh with nverts {}", nverts);
        return;
    }

    let nverts = nverts as usize;
    let p = unsafe { core::slice::from_raw_parts(p, nverts) };
    let n = if n.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(n, nverts) })
    };
    let st = if st.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(st, nverts) })
    };
    let cs = if cs.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(cs, nverts) })
    };
    let indexes = unsafe { core::slice::from_raw_parts(indexes, ntris as usize) };

    let hash = {
        let mut hasher = std::hash::DefaultHasher::new();
        let p: &[u8] = bytemuck::cast_slice(p);
        p.hash(&mut hasher);
        let n: &[u8] = n.map_or_default(|n| bytemuck::cast_slice(n));
        n.hash(&mut hasher);
        let st: &[u8] = st.map_or_default(|st| bytemuck::cast_slice(st));
        st.hash(&mut hasher);
        let cs: &[u8] = cs.map_or_default(|cs| bytemuck::cast_slice(cs));
        cs.hash(&mut hasher);
        indexes.hash(&mut hasher);
        hasher.finish()
    };

    let ctm = transform::get();
    let mut os = OBJECTSTATE.lock().unwrap();
    let cutout = os.cutout;
    let cutoutparams = os.cutoutparams.clone();
    let material = Arc::new(os.last_material().clone());

    if !os
        .world
        .mesh_instance(hash, ctm, cutout, cutoutparams.clone(), material.clone())
    {
        os.world.mesh_create(
            hash,
            ctm,
            cutout,
            cutoutparams,
            material,
            Mesh::tricreate(
                indexes,
                p.to_vec(),
                n.map(|n| n.to_vec()),
                cs.map(|cs| cs.to_vec()),
                st.map(|st| st.to_vec()),
            ),
        );
    }
}
