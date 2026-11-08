use ultraviolet::Vec3;

mod screen;
mod shaders;
mod transform;
mod world;

pub(crate) use screen::{bsdf_test, warp_test};
pub use shaders::*;
pub(crate) use world::WorldState;

type RtFloat = f32;
type RtInt = i32;
type RtColor = Vec3;
type RtPoint = Vec3;
type RtVector = Vec3;
type RtNormal = Vec3;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq, Debug)]
pub struct RtMaterial {
    /// kd: diffuse reflectance per channel, 0 to 1
    pub albedo: RtColor,

    /// ks: the reflectance head on: it weights the mirror ray and the highlight (HW2), the glossy lobe (HW3), and is Schlick's F0, metal or not (HW4)
    pub specular: RtColor,

    pub roughness: RtFloat, /* 0 = mirror, 1 = fully rough; the lobe's width is alpha = roughness^2 in every
                            homework. HW2: the Phong exponent n = 2 / alpha^2 - 2 (alpha clamped to
                            [0.05, 1]) and how much of the mirror ray survives; HW3: the glossy lobe's
                            width; HW4: GGX's alpha */
    pub metallic: RtFloat, /* 0 = dielectric, 1 = metal. HW4 only; HW2 and HW3 ignore it */
    pub transmission: RtFloat, /* kt: 0 = opaque, 1 = clear glass */
    pub ior: RtFloat,      /* eta: index of refraction of the inside (1.5 = glass) */
    pub emission: RtColor, /* radiance emitted (seen directly, this bright). Nonzero makes the draw call a
                           light: a point light at its center in HW2, its surface sampled in HW3, with
                           MIS in HW4 */
}

impl RtMaterial {
    /// The default material: albedo 0.8, no specular, roughness 0.5, opaque, ior 1.5, no emission.
    pub const fn default_material() -> RtMaterial {
        RtMaterial {
            albedo: Vec3::broadcast(0.8),
            specular: Vec3::broadcast(0.),
            roughness: 0.5,
            metallic: 0.0,
            transmission: 0.0,
            ior: 1.5,
            emission: Vec3::broadcast(0.),
        }
    }
}
