pub mod debug;
pub mod ggx_albedo;
pub mod hashing;
pub mod image;
pub mod noise;
pub mod samplers;
pub mod texture;
pub mod timer;
pub mod warp;

pub fn factor(n: u16) -> u16 {
    let mut factor = 1;
    for x in (2..=n.isqrt()).rev() {
        if n % x == 0 {
            factor = x;
            break;
        }
    }
    debug_assert_eq!(factor * (n / factor), n);
    factor
}
