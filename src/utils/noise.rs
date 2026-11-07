//! noise.rs: perlinNoise (shaders.h), provided complete in both kits
//! (noise.cpp, noise.rs). Ken Perlin's improved noise (2002), as HW1 asked
//! for: his permutation of 0..255, his fade curve 6t^5 - 15t^4 + 10t^3 and
//! his gradient function. Signed, about [-1, 1], zero at integer points,
//! the lattice index wrapped into 0..255. Given rather than brought over
//! from HW1 so every renderer computes the same noise and the benchmark
//! times the same work for everyone. The scenes call it too, through the C
//! name exported at the end.

#![allow(non_snake_case)] // the interface's name

/// Perlin's permutation, from his reference implementation
/// (https://mrl.cs.nyu.edu/~perlin/noise/). Indexed modulo 256.
const PERM: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69,
    142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219,
    203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122, 60, 211, 133, 230,
    220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73, 209, 76,
    132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173,
    186, 3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206,
    59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163,
    70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232,
    178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162,
    241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184, 84, 204,
    176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141,
    128, 195, 78, 66, 215, 61, 156, 180,
];

#[inline]
fn perm(i: i32) -> i32 {
    PERM[(i & 255) as usize] as i32
}
#[inline]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}
#[inline]
fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

/// The gradient for a hash: one of the 12 edge directions of a cube (plus
/// four repeats), dotted with (x, y, z).
#[inline]
fn grad(hash: i32, x: f64, y: f64, z: f64) -> f64 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

#[inline]
pub fn perlin_noise(x: f64, y: f64, z: f64) -> f64 {
    let (fx, fy, fz) = (x.floor(), y.floor(), z.floor());
    let (xi, yi, zi) = ((fx as i32) & 255, (fy as i32) & 255, (fz as i32) & 255);
    let (x, y, z) = (x - fx, y - fy, z - fz);
    let (u, v, w) = (fade(x), fade(y), fade(z));
    let a = perm(xi) + yi;
    let aa = perm(a) + zi;
    let ab = perm(a + 1) + zi;
    let b = perm(xi + 1) + yi;
    let ba = perm(b) + zi;
    let bb = perm(b + 1) + zi;
    lerp(
        w,
        lerp(
            v,
            lerp(u, grad(perm(aa), x, y, z), grad(perm(ba), x - 1.0, y, z)),
            lerp(
                u,
                grad(perm(ab), x, y - 1.0, z),
                grad(perm(bb), x - 1.0, y - 1.0, z),
            ),
        ),
        lerp(
            v,
            lerp(
                u,
                grad(perm(aa + 1), x, y, z - 1.0),
                grad(perm(ba + 1), x - 1.0, y, z - 1.0),
            ),
            lerp(
                u,
                grad(perm(ab + 1), x, y - 1.0, z - 1.0),
                grad(perm(bb + 1), x - 1.0, y - 1.0, z - 1.0),
            ),
        ),
    )
}

/// The C name the scenes call (shaders.h).
#[unsafe(no_mangle)]
pub extern "C" fn perlinNoise(x: f64, y: f64, z: f64) -> f64 {
    perlin_noise(x, y, z)
}
