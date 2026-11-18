//! sampler.rs: your samplers (the "Samplers" handout; workflow steps 2, 4
//! and 5). The interface at the top is given: keep its names, since the
//! given warptest.rs calls make_sampler. Below it, write each sampler as
//! a type that implements Sampler, built on the given hashing.rs, and
//! make make_sampler return the one a name asks for.

use ultraviolet::Vec2;

use crate::utils::hashing::*;

/// A pixel sample's random numbers, dimension by dimension, in the
/// handout's order. Every value depends only on the pixel, the sample's
/// index, the dimension and the seed, never on the thread.
pub trait Sampler: Send {
    fn make(spp: u16, seed: u64) -> Self
    where
        Self: Sized;

    /// Start sample `index` of the pixel at (x, y), in the full frame's
    /// coordinates: from here next_1d and next_2d give that sample's
    /// dimensions in order.
    fn start_pixel_sample(&mut self, x: i32, y: i32, index: i32);
    #[allow(dead_code)] // a renderer may draw no 1D dimension (the time is extra credit)
    fn next_1d(&mut self) -> f32;
    /// Two sample values, each in [0, 1).
    fn next_2d(&mut self) -> Vec2;
}

/// The sampler called `name` ("independent", "stratified", "halton",
/// "sobol") for spp samples per pixel and the given seed. TODO (workflow
/// step 2): until you write it, it stops the program.
pub fn make_sampler(name: &str, spp: i32, seed: u64) -> Box<dyn Sampler> {
    let spp = spp as u16;
    match name {
        "independent" => Box::new(Independent::make(spp, seed)),
        "stratified" => Box::new(Stratified::make(spp, seed)),
        "halton" => Box::new(Halton::make(spp, seed)),
        _ => unimplemented!(),
    }
}

struct Independent {
    rng: Pcg32,
    seed: u64,
}
impl Sampler for Independent {
    fn make(_: u16, seed: u64) -> Self
    where
        Self: Sized,
    {
        Self {
            rng: Pcg32::new(),
            seed,
        }
    }

    #[inline]
    fn start_pixel_sample(&mut self, x: i32, y: i32, index: i32) {
        start_stream(&mut self.rng, x, y, index, self.seed);
    }

    #[inline]
    fn next_1d(&mut self) -> f32 {
        self.rng.uniform()
    }

    #[inline]
    fn next_2d(&mut self) -> Vec2 {
        Vec2::new(self.rng.uniform(), self.rng.uniform())
    }
}

struct Stratified {
    rng: Pcg32,
    seed: u64,

    strata: (u8, u8),
    index: u16,
    hash: u64,
    dimension: i32,
}
impl Sampler for Stratified {
    fn make(spp: u16, seed: u64) -> Self
    where
        Self: Sized,
    {
        let mut factor = 1;
        for x in (2..spp.isqrt()).rev() {
            if spp % x == 0 {
                factor = x;
                break;
            }
        }
        debug_assert_eq!(factor * (spp / factor), spp);

        Self {
            rng: Pcg32::new(),
            seed,

            strata: (factor as u8, (spp / factor) as u8),
            index: 0,
            hash: 0,
            dimension: 0,
        }
    }

    #[inline]
    fn start_pixel_sample(&mut self, x: i32, y: i32, index: i32) {
        self.index = index as _;
        self.hash = hash3(x as _, y as _, self.seed);
        self.dimension = 0;
        start_stream(&mut self.rng, x, y, index, self.seed);
    }

    #[inline]
    fn next_1d(&mut self) -> f32 {
        let key = low32(hash_dimension(self.hash, self.dimension));
        let p = permutation_element(self.index as _, self.spp(), key) as f32;
        self.dimension += 1;
        ((self.rng.uniform() + p) / (self.spp() as f32)).min(ONE_MINUS_EPSILON)
    }

    #[inline]
    fn next_2d(&mut self) -> Vec2 {
        let key = low32(hash_dimension(self.hash, self.dimension));
        let p = permutation_element(self.index as _, self.spp(), key) as f32;
        self.dimension += 2;
        Vec2::new(
            ((p % self.strata.0 as f32) + self.rng.uniform()) / self.strata.0 as f32,
            ((p / self.strata.0 as f32) + self.rng.uniform()) / self.strata.1 as f32,
        )
        .min_by_component(Vec2::broadcast(ONE_MINUS_EPSILON))
    }
}
impl Stratified {
    #[inline]
    const fn spp(&self) -> u32 {
        self.strata.0 as u32 * self.strata.1 as u32
    }
}

struct Halton {
    rng: Pcg32,
    seed: u64,

    index: i32,
    hash: u64,
    dimension: i32,
}
impl Sampler for Halton {
    fn make(_: u16, seed: u64) -> Self
    where
        Self: Sized,
    {
        Self {
            rng: Pcg32::new(),
            seed,

            index: 0,
            hash: 0,
            dimension: 0,
        }
    }

    fn start_pixel_sample(&mut self, x: i32, y: i32, index: i32) {
        self.index = index;
        self.hash = hash3(x as _, y as _, self.seed);
        self.dimension = 0;
        start_stream(&mut self.rng, x, y, index, self.seed);
    }

    fn next_1d(&mut self) -> f32 {
        let dim = self.dim();
        self.dimension = dim as i32 + 1;
        dim
    }

    fn next_2d(&mut self) -> Vec2 {
        let dim = self.dim();
        self.dimension = dim as i32 + 2;
        Vec2::new(dim, dim + 1.)
    }
}
impl Halton {
    fn dim(&self) -> f32 {
        let h = hash_dimension(self.hash, self.dimension);
        if self.dimension & 1 == 0 {
            fixed_to_float(owen_scramble(
                self.index.reverse_bits().cast_unsigned(),
                low32(h),
            ))
        } else {
            owen_scrambled_radical_inverse(
                PRIME_TABLE[(self.dimension % 1024).cast_unsigned() as usize],
                self.index as _,
                h,
            )
        }
    }
}
