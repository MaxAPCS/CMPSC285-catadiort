use ultraviolet::{Vec2, Vec2x4};
use wide::{f32x4, u32x4};

use crate::utils::hashing::*;

trait Sampler: Send {
    fn pixel(x: u32, y: u32, n: u32, spp: u16, seed: u64) -> Self
    where
        Self: Sized;
    fn next1(&mut self) -> f32;
    fn next2(&mut self) -> Vec2;
}

#[derive(Clone, Default)]
pub struct AnySamplerx4([AnySampler; 4]);
impl AnySamplerx4 {
    pub fn pixel(
        x: u32x4,
        y: u32x4,
        n: u32x4,
        spp: u16,
        seed: u64,
        sampler: crate::options::Sampler,
    ) -> Self {
        let x = x.as_array_ref();
        let y = y.as_array_ref();
        let n = n.as_array_ref();
        Self(core::array::from_fn(|i| {
            AnySampler::pixel(x[i] as _, y[i] as _, n[i] as _, spp, seed, sampler)
        }))
    }

    #[inline]
    pub fn next1(&mut self) -> f32x4 {
        core::array::from_fn(|i| self.0[i].next1()).into()
    }

    #[inline]
    pub fn next2(&mut self) -> Vec2x4 {
        core::array::from_fn(|i| self.0[i].next2()).into()
    }
}
impl Into<[AnySampler; 4]> for AnySamplerx4 {
    fn into(self) -> [AnySampler; 4] {
        self.0
    }
}
impl From<[AnySampler; 4]> for AnySamplerx4 {
    fn from(value: [AnySampler; 4]) -> Self {
        Self(value)
    }
}

#[derive(Clone)]
pub enum AnySampler {
    Center(Center),
    Independent(Independent),
    Stratified(Stratified),
    Halton(Halton),
    Sobol(Sobol),
}
impl AnySampler {
    #[inline]
    pub(super) fn pixel(
        x: u32,
        y: u32,
        n: u32,
        spp: u16,
        seed: u64,
        sampler: crate::options::Sampler,
    ) -> Self {
        match sampler {
            crate::options::Sampler::Center => Self::Center(Center::pixel(x, y, n, spp, seed)),
            crate::options::Sampler::Independent => {
                Self::Independent(Independent::pixel(x, y, n, spp, seed))
            }
            crate::options::Sampler::Stratified => {
                Self::Stratified(Stratified::pixel(x, y, n, spp, seed))
            }
            crate::options::Sampler::Halton => Self::Halton(Halton::pixel(x, y, n, spp, seed)),
            crate::options::Sampler::Sobol => Self::Sobol(Sobol::pixel(x, y, n, spp, seed)),
        }
    }

    #[inline]
    fn next1(&mut self) -> f32 {
        match self {
            Self::Center(inner) => inner.next1(),
            Self::Independent(inner) => inner.next1(),
            Self::Stratified(inner) => inner.next1(),
            Self::Halton(inner) => inner.next1(),
            Self::Sobol(inner) => inner.next1(),
        }
    }

    #[inline]
    pub(super) fn next2(&mut self) -> Vec2 {
        match self {
            Self::Center(inner) => inner.next2(),
            Self::Independent(inner) => inner.next2(),
            Self::Stratified(inner) => inner.next2(),
            Self::Halton(inner) => inner.next2(),
            Self::Sobol(inner) => inner.next2(),
        }
    }
}
impl Default for AnySampler {
    fn default() -> Self {
        Self::Center(Center)
    }
}

#[derive(Clone)]
pub struct Center;
impl Sampler for Center {
    #[inline]
    fn pixel(_x: u32, _y: u32, _n: u32, _spp: u16, _seed: u64) -> Self
    where
        Self: Sized,
    {
        Self
    }

    #[inline]
    fn next1(&mut self) -> f32 {
        0.5
    }

    #[inline]
    fn next2(&mut self) -> Vec2 {
        Vec2::broadcast(0.5)
    }
}

#[derive(Clone)]
pub struct Independent {
    rng: Pcg32,
    seed: u64,
}
impl Sampler for Independent {
    fn pixel(x: u32, y: u32, n: u32, _spp: u16, seed: u64) -> Self
    where
        Self: Sized,
    {
        let mut nself = Self {
            rng: Pcg32::new(),
            seed,
        };
        nself
            .rng
            .set_sequence(hash3(x as _, y as _, nself.seed), nself.seed);
        nself.rng.advance((n as u64).wrapping_mul(65536));
        nself
    }

    #[inline]
    fn next1(&mut self) -> f32 {
        self.rng.uniform()
    }

    #[inline]
    fn next2(&mut self) -> Vec2 {
        Vec2::new(self.rng.uniform(), self.rng.uniform())
    }
}

#[derive(Clone)]
pub struct Stratified {
    rng: Pcg32,
    seed: u64,

    strata: (u16, u16),
    index: u16,
    hash: u64,
    dimension: i32,
}
impl Sampler for Stratified {
    fn pixel(x: u32, y: u32, n: u32, spp: u16, seed: u64) -> Self
    where
        Self: Sized,
    {
        let mut factor = 1;
        for x in (2..=spp.isqrt()).rev() {
            if spp % x == 0 {
                factor = x;
                break;
            }
        }
        debug_assert_eq!(factor * (spp / factor), spp);
        let mut nself = Self {
            rng: Pcg32::new(),
            seed,

            strata: (factor, (spp / factor)),
            index: n as _,
            hash: hash3(x as _, y as _, seed),
            dimension: 0,
        };
        nself.rng.set_sequence(nself.hash, nself.seed);
        nself.rng.advance((n as u64).wrapping_mul(65536));
        nself
    }

    #[inline]
    fn next1(&mut self) -> f32 {
        let key = low32(hash_dimension(self.hash, self.dimension));
        let p = permutation_element(self.index as _, self.spp(), key) as f32;
        self.dimension += 1;
        ((self.rng.uniform() + p) / (self.spp() as f32)).min(ONE_MINUS_EPSILON)
    }

    #[inline]
    fn next2(&mut self) -> Vec2 {
        let key = low32(hash_dimension(self.hash, self.dimension));
        let p = permutation_element(self.index as _, self.spp(), key);
        self.dimension += 2;
        Vec2::new(
            ((p % self.strata.0 as u32) as f32 + self.rng.uniform()) / self.strata.0 as f32,
            ((p / self.strata.0 as u32) as f32 + self.rng.uniform()) / self.strata.1 as f32,
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

#[derive(Clone)]
pub struct Halton {
    rng: Pcg32,
    seed: u64,

    index: u16,
    hash: u64,
    dimension: i32,
}
impl Sampler for Halton {
    fn pixel(x: u32, y: u32, n: u32, _spp: u16, seed: u64) -> Self
    where
        Self: Sized,
    {
        let mut nself = Self {
            rng: Pcg32::new(),
            seed,

            index: n as _,
            hash: hash3(x as _, y as _, seed),
            dimension: 0,
        };
        nself.rng.set_sequence(nself.hash, nself.seed);
        nself.rng.advance((n as u64).wrapping_mul(65536));
        nself
    }

    #[inline]
    fn next1(&mut self) -> f32 {
        let dim = self.dim();
        self.dimension += 1;
        dim
    }

    #[inline]
    fn next2(&mut self) -> Vec2 {
        Vec2::new(self.next1(), self.next1())
    }
}
impl Halton {
    fn dim(&self) -> f32 {
        let h = hash_dimension(self.hash, self.dimension);
        if self.dimension & 1 == 0 {
            fixed_to_float(owen_scramble((self.index as u32).reverse_bits(), low32(h)))
        } else {
            owen_scrambled_radical_inverse(
                PRIME_TABLE[(self.dimension % 1024).cast_unsigned() as usize],
                self.index as _,
                h,
            )
        }
    }
}

#[derive(Clone)]
pub struct Sobol {
    rng: Pcg32,
    seed: u64,

    index: u16,
    hash: u64,
    dimension: i32,
}
impl Sampler for Sobol {
    fn pixel(x: u32, y: u32, n: u32, _spp: u16, seed: u64) -> Self
    where
        Self: Sized,
    {
        let mut nself = Self {
            rng: Pcg32::new(),
            seed,

            index: n as _,
            hash: hash3(x as _, y as _, seed),
            dimension: 0,
        };
        nself.rng.set_sequence(nself.hash, nself.seed);
        nself.rng.advance((n as u64).wrapping_mul(65536));
        nself
    }

    #[inline]
    fn next1(&mut self) -> f32 {
        let dim = self.dim();
        self.dimension += 1;
        dim.x
    }

    #[inline]
    fn next2(&mut self) -> Vec2 {
        let dim = self.dim();
        self.dimension += 2;
        dim
    }
}
impl Sobol {
    fn dim(&mut self) -> Vec2 {
        let h = hash_dimension(self.hash, self.dimension);
        let i = owen_scramble(self.index as _, low32(h));
        self.index = i as _;
        Vec2::new(
            fixed_to_float(owen_scramble(i.reverse_bits(), high32(h))),
            fixed_to_float(owen_scramble(sobol2(i), low32(mix64(h)))),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sobol() {
        assert_eq!(
            Sobol::pixel(0, 0, 0, 0, 0).next2(),
            Vec2::new(0.182183221, 0.827739835)
        );
        assert_eq!(
            Sobol::pixel(0, 0, 1, 0, 0).next2(),
            Vec2::new(0.834863305, 0.179580197)
        );
        assert_eq!(
            Sobol::pixel(0, 0, 2, 0, 0).next2(),
            Vec2::new(0.556520045, 0.618239522)
        );
        assert_eq!(
            Sobol::pixel(0, 0, 3, 0, 0).next2(),
            Vec2::new(0.357120842, 0.266228676)
        );
    }
}
