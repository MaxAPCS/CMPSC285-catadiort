use std::{
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};
use ultraviolet::{Mat4, Vec3};

use super::{transform, *};
use crate::engine::{
    DirectionalLight, Material, Mesh, MeshEmissive, PointLight, Sphere, SpotLight, World,
    from_emissive,
};

pub(crate) struct WorldState {
    pub shader: Option<unsafe extern "C" fn(*mut RtShadeContext)>,
    pub shaderparams: Option<Arc<[u8]>>,
    pub cutout: Option<unsafe extern "C" fn(*mut RtShadeContext) -> RtInt>,
    pub cutoutparams: Option<Arc<[u8]>>,

    material: usize,
    world: World,
}

impl WorldState {
    const fn new() -> Self {
        Self {
            shader: None,
            shaderparams: None,
            cutout: None,
            cutoutparams: None,

            material: 0,
            world: World::empty(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    fn set_material(&mut self, material: Option<RtMaterial>) {
        let Some(material) = material else {
            self.material = 0;
            debug_assert_eq!(self.get_material(), RtMaterial::default_material());
            return;
        };
        self.material = self.world.materials.len();
        self.world.materials.push(Material::from(
            material,
            self.shader,
            self.shaderparams.clone(),
        ));
        debug_assert_eq!(self.get_material(), material);
    }

    fn rebind_material(&mut self) {
        self.set_material(Some(self.get_material()));
    }

    pub fn get_materialidx(&self) -> usize {
        self.material
    }

    fn get_material(&self) -> RtMaterial {
        (&self.world.materials[self.material]).into()
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
pub extern "C" fn rtDirectionalLight(direction: &Vec3, irradiance: &RtColor) {
    let dir = transform::get().transform_vec3(*direction).normalized();

    let obj = DirectionalLight::new(dir, *irradiance);
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
    let obj = Sphere::create(&mut os, ctm * Mat4::from_scale(radius));
    if let Some(pl) = PointLight::from_emissive(&obj, &os.get_material()) {
        os.world.pointlights.push(pl);
    }
    os.world.spheres.push(obj);
}

/// A mesh is baked instead: its vertices go through the CTM
/// now, its normals through the inverse transpose, and only the inverse is
/// kept, for Po (the hit point in object space, which solid shaders read)
/// and the tangents. What the primitives of one call share (the copy, its
/// inverse, the material and the shaders bound at the call) is one
/// draw-call record they name by index (Intersection handout).

/// ntris triangles over nverts vertices, counterclockwise from outside; N,
/// st and Cs may be null. Vertices go through the CTM, normals through its
/// inverse transpose; each triangle keeps its first vertex, two edges and
/// constant tangents (Intersection handout). A mirroring CTM (a negative
/// determinant, ctm().determinant3()) swaps the winding so the outside
/// stays outside; a bad index skips the triangle.
/// # Safety
/// The arrays are as long as nverts and 3 ntris say; N, st and Cs may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rtTriangleMesh(
    nverts: RtInt,
    p: *const RtPoint,
    n: *const RtNormal,
    st: *const (RtFloat, RtFloat),
    cs: *const RtColor,
    ntris: RtInt,
    indexes: *const [RtInt; 3],
) {
    if nverts <= 0 || ntris <= 0 || p.is_null() || indexes.is_null() {
        #[cfg(debug_assertions)]
        println!("rtobject: skipped rtTriangleMesh with nverts {}", nverts);
        return;
    }

    let hash = {
        let mut hasher = std::hash::DefaultHasher::new();
        p.hash(&mut hasher);
        n.hash(&mut hasher);
        st.hash(&mut hasher);
        cs.hash(&mut hasher);
        indexes.hash(&mut hasher);
        hasher.finish()
    };

    let nverts = nverts as usize;
    let p = unsafe { core::slice::from_raw_parts(p, nverts) }.to_vec();
    let n = if n.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(n, nverts) }.to_vec())
    };
    let st = if st.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(st, nverts) }.to_vec())
    };
    let cs = if cs.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(cs, nverts) }.to_vec())
    };
    let indexes = unsafe { core::slice::from_raw_parts(indexes, ntris as usize) };

    let ctm = transform::get();

    let mut os = OBJECTSTATE.lock().unwrap();
    let obj = Mesh::create(&mut os, ctm, hash, indexes, p, n, cs, st);
    match from_emissive(&obj, &os.get_material()) {
        Some(MeshEmissive::Plane(pl)) => os.world.planelights.push(pl),
        Some(MeshEmissive::Point(pl)) => os.world.pointlights.push(pl),
        None => {}
    }
    os.world.meshes.push(obj);
}
