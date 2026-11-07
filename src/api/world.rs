use std::sync::{Arc, Mutex};
use ultraviolet::{Mat4, Vec3};

use super::transform;
use super::*;
use crate::engine::{DirectionalLight, PointLight, Sphere, SpotLight, World};

pub(crate) struct WorldState {
    pub material: RtMaterial,
    pub shader: Option<unsafe extern "C" fn(*mut RtShadeContext)>,
    pub shaderparams: Option<Arc<[u8]>>,
    pub cutout: Option<unsafe extern "C" fn(*mut RtShadeContext) -> RtInt>,
    pub cutoutparams: Option<Arc<[u8]>>,

    world: World,
}

impl WorldState {
    const fn new() -> Self {
        Self {
            material: RtMaterial::default_material(),
            shader: None,
            shaderparams: None,
            cutout: None,
            cutoutparams: None,

            world: World::empty(),
        }
    }
}

static OBJECTSTATE: Mutex<WorldState> = Mutex::new(WorldState::new());

#[inline]
pub(super) fn take() -> World {
    core::mem::take(&mut OBJECTSTATE.lock().unwrap().world)
}

#[inline]
pub(super) fn clear() {
    OBJECTSTATE.lock().unwrap().world.clear();
}

/* attributes ------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub extern "C" fn rtMaterial(m: *const RtMaterial) {
    if m.is_null() {
        return;
    }
    // Safety: `m` is null or points at an RtMaterial.
    OBJECTSTATE.lock().unwrap().material = unsafe { *m };
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
    }
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
    let obj = Sphere::create(&os, ctm * Mat4::from_scale(radius));
    if let Some(pl) = PointLight::from_emissive(&obj) {
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
    st: *const RtFloat,
    cs: *const RtColor,
    ntris: RtInt,
    indices: *const RtInt,
) {
    if nverts <= 0 || ntris <= 0 || p.is_null() || indices.is_null() {
        #[cfg(debug_assertions)]
        println!("rtobject: skipped rtTriangleMesh with nverts {}", nverts);
        return;
    }
    let _ = (n, st, cs);
    /* TODO: add a draw call, then bake the mesh to world space and add it and
     * its triangles to their lists (Workflow, step 7) */
}
