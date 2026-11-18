//! warp.rs: your warps (the "Warps" handout; workflow step 6). Write each
//! function as the handout derives it, keeping its name and signature: the
//! given warptest.rs calls them. Directions are in a local frame about +z.
//! A constant density takes no point; the others take the direction's
//! cosine (and the lobe its exponent). The caller checks the support.
//! Until you write a warp, it stops the program when called.

use ultraviolet::{Vec2, Vec3};

/// A point in the unit disk, (x, y, 0), by Shirley and Chiu's concentric
/// map. Density 1 / pi over the disk's area.
pub fn sample_concentric_disk(uv: Vec2) -> Vec3 {
    let ab = 2. * uv - Vec2::one();
    if ab == Vec2::zero() {
        return Vec3::zero();
    }
    let (r, phi) = if ab.x.abs() > ab.y.abs() {
        (ab.x, core::f32::consts::FRAC_PI_4 * (ab.y / ab.x))
    } else {
        (
            ab.y,
            core::f32::consts::FRAC_PI_2 - core::f32::consts::FRAC_PI_4 * (ab.x / ab.y),
        )
    };
    let (s, c) = phi.sin_cos();
    Vec3::new(r * c, r * s, 0.)
}
pub const fn concentric_disk_pdf() -> f32 {
    core::f32::consts::FRAC_1_PI
}

/// A direction in the hemisphere about +z with density cos(theta) / pi,
/// by Malley's method.
pub fn sample_cosine_hemisphere(uv: Vec2) -> Vec3 {
    let mut out = sample_concentric_disk(uv);
    out.z = (1. - out.x * out.x - out.y * out.y).max(0.).sqrt();
    out
}
pub const fn cosine_hemisphere_pdf(cos_theta: f32) -> f32 {
    cos_theta.max(0.) * concentric_disk_pdf()
}

#[inline]
fn archimedes(z: f32, phi: f32) -> Vec3 {
    let zs = (1. - z * z).sqrt();
    let (s, c) = phi.sin_cos();
    Vec3::new(zs * c, zs * s, z)
}

/// A direction uniform on the unit sphere. Density 1 / (4 pi).
pub fn sample_uniform_sphere(uv: Vec2) -> Vec3 {
    let z = 1. - 2. * uv.x;
    let phi = 2. * core::f32::consts::PI * uv.y;
    archimedes(z, phi)
}
pub const fn uniform_sphere_pdf() -> f32 {
    core::f32::consts::FRAC_1_PI / 4.
}

/// A direction uniform in the cone about +z whose half-angle has cosine
/// cos_theta_max (in [-1, 1)): the sphere light's sampler.
pub fn sample_uniform_cone(uv: Vec2, cos_theta_max: f32) -> Vec3 {
    let z = 1. - uv.x * (1. - cos_theta_max);
    let phi = 2. * core::f32::consts::PI * uv.y;
    archimedes(z, phi)
}
pub const fn uniform_cone_pdf(cos_theta_max: f32) -> f32 {
    core::f32::consts::FRAC_1_PI / (2. * (1. - cos_theta_max))
}

/// Barycentric coordinates (b0, b1, b2) uniform over a triangle. Density 2
/// over the (b0, b1) plane.
pub fn sample_uniform_triangle(uv: Vec2) -> Vec3 {
    let s = uv.x.sqrt();
    Vec3::new(1. - s, uv.y * s, (1. - uv.y) * s)
}
pub const fn uniform_triangle_pdf() -> f32 {
    2.
}

/// A direction about +z from the normalized Phong lobe of exponent n.
pub fn sample_phong_lobe(uv: Vec2, n: f32) -> Vec3 {
    let z = uv.x.powf((n + 1.).recip());
    let phi = 2. * core::f32::consts::PI * uv.y;
    archimedes(z, phi)
}
pub fn phong_lobe_pdf(cos_theta: f32, n: f32) -> f32 {
    cos_theta.powf(n) * (n + 1.) * core::f32::consts::FRAC_1_PI / 2.
}

pub mod test {
    //! warptest.rs: --warptest WARP N [PARAM] (the "Warps" handout, Section
    //! 4), given complete: N samples of a warp, one "x y z pdf" line each ("x
    //! y 0 pdf" for the disk, "b0 b1 b2 pdf" for the triangle), each pdf
    //! computed from its point, for tools/warptest.py's chi-square test and
    //! pictures. The 2D samples are the N samples of pixel (0, 0) under
    //! --sampler, independent unless given. It calls your sampler and your
    //! warps by the names sampler.rs and warp.rs fix; your warp_test in rt.rs
    //! calls it.

    use std::io::{BufWriter, Write};

    use super::*;
    use crate::options::Options;
    use crate::utils::sampler::make_sampler;
    pub fn run_warp_test(o: &Options) -> i32 {
        let w = o.warp.as_str();
        let param = o.warp_param;
        if w == "uniform_cone" && !(param >= -1.0 && param < 1.0) {
            eprintln!(
                "--warptest uniform_cone: cosThetaMax must be in [-1, 1), not {}",
                param
            );
            return 1;
        }
        if w == "phong" && !(param >= 0.0) {
            eprintln!(
                "--warptest phong: the exponent must be at least 0, not {}",
                param
            );
            return 1;
        }
        let name = if o.sampler.is_empty() {
            "independent"
        } else {
            o.sampler.as_str()
        };
        let mut sampler = make_sampler(name, o.warp_count, o.seed);
        let stdout = std::io::stdout();
        let mut out = BufWriter::with_capacity(1 << 16, stdout.lock());
        for i in 0..o.warp_count {
            sampler.start_pixel_sample(0, 0, i);
            let u = sampler.next_2d();
            let (p, pdf) = match w {
                "disk" => (sample_concentric_disk(u), concentric_disk_pdf()),
                "cosine_hemisphere" => {
                    let p = sample_cosine_hemisphere(u);
                    (p, cosine_hemisphere_pdf(p.z))
                }
                "uniform_sphere" => (sample_uniform_sphere(u), uniform_sphere_pdf()),
                "uniform_cone" => {
                    let p = sample_uniform_cone(u, param);
                    let pdf = if p.z >= param {
                        uniform_cone_pdf(param)
                    } else {
                        0.0
                    };
                    (p, pdf)
                }
                "triangle" => (sample_uniform_triangle(u), uniform_triangle_pdf()),
                "phong" => {
                    let p = sample_phong_lobe(u, param);
                    (p, phong_lobe_pdf(p.z, param))
                }
                _ => {
                    eprintln!("--warptest: no warp '{}'", w);
                    return 1;
                }
            };
            if writeln!(out, "{} {} {} {}", p.x, p.y, p.z, pdf).is_err() {
                return 1;
            }
        }
        if out.flush().is_err() {
            return 1;
        }
        0
    }
}
