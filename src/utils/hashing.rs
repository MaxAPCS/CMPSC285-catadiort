//! hashing.rs: the hash and random-number primitives every sampler is
//! built on (HW3 handout), the twin of the C++ kit's `src/hashing.h`.
//! Provided complete in the HW3 kit, so that a sampler written to the HW3
//! handout gives the reference's numbers bit for bit. Nothing here is
//! specific to rendering. Declare it in `main.rs` beside the other
//! modules (`mod hashing;`); the names are the handout's in
//! snake_case (`hashDimension` is `hash_dimension`, `Pcg32::setSequence`
//! is `set_sequence`, and so on).
#![allow(dead_code)]

/// The largest f32 below 1, so a sample is always in [0, 1).
pub const ONE_MINUS_EPSILON: f32 = 1.0 - f32::EPSILON / 2.0;

const fn first_primes<const N: usize>() -> [u32; N] {
    const LIMIT: usize = 9_100;

    // Sieve of Eratosthenes
    let mut composite = [false; LIMIT + 1];
    let mut p = 2usize;
    while p * p <= LIMIT {
        if !composite[p] {
            let mut m = p * p;
            while m <= LIMIT {
                composite[m] = true;
                m += p;
            }
        }
        p += 1;
    }

    // Take first N
    let mut primes = [0u32; N];
    let mut count = 0usize;
    let mut candidate = 2usize;
    while count < N {
        if candidate > LIMIT {
            panic!("sieve LIMIT too small to hold N primes"); // compile-time error
        }
        if !composite[candidate] {
            primes[count] = candidate as u32;
            count += 1;
        }
        candidate += 1;
    }
    primes
}

pub const PRIME_TABLE: [u32; 1024] = first_primes();

/// The SplitMix64 finalizer: a 64-bit hash of a 64-bit integer.
pub fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

/// A hash of three integers: a pixel and a seed into a stream number.
pub fn hash3(a: u64, b: u64, c: u64) -> u64 {
    mix64(
        a.wrapping_mul(0x9e3779b97f4a7c15)
            ^ mix64(b.wrapping_add(0x632be59bd9b4e019))
            ^ mix64(c.wrapping_add(0x85157af5)),
    )
}

/// The hash of one dimension of one pixel's samples, from the pixel's hash.
pub fn hash_dimension(pixel_hash: u64, dimension: i32) -> u64 {
    mix64(
        pixel_hash
            ^ mix64(
                (dimension as u32 as u64)
                    .wrapping_mul(0x9e3779b97f4a7c15)
                    .wrapping_add(0x7f4a7c15),
            ),
    )
}

pub fn low32(h: u64) -> u32 {
    h as u32
}

pub fn high32(h: u64) -> u32 {
    (h >> 32) as u32
}

/// A 32-bit fixed-point number (bit 31 the first binary digit) as an f32
/// in [0, 1).
#[inline]
pub fn fixed_to_float(x: u32) -> f32 {
    (x as f32 * const { (u32::MAX as f32 + 1.).recip() }).min(ONE_MINUS_EPSILON)
}

/// Element i of a random permutation of 0 .. n - 1 chosen by `hash`
/// (Kensler 2013, "Correlated Multi-Jittered Sampling"): a hash that is a
/// bijection on the integers below the next power of two above n, cycled
/// until it lands below n.
pub fn permutation_element(mut i: u32, n: u32, hash: u32) -> u32 {
    let mut w = n - 1;
    w |= w >> 1;
    w |= w >> 2;
    w |= w >> 4;
    w |= w >> 8;
    w |= w >> 16;
    loop {
        i ^= hash;
        i = i.wrapping_mul(0xe170893d);
        i ^= hash >> 16;
        i ^= (i & w) >> 4;
        i ^= hash >> 8;
        i = i.wrapping_mul(0x0929eb3f);
        i ^= hash >> 23;
        i ^= (i & w) >> 1;
        i = i.wrapping_mul(1 | (hash >> 27));
        i = i.wrapping_mul(0x6935fa69);
        i ^= (i & w) >> 11;
        i = i.wrapping_mul(0x74dcb303);
        i ^= (i & w) >> 2;
        i = i.wrapping_mul(0x9e501cc3);
        i ^= (i & w) >> 2;
        i = i.wrapping_mul(0xc860a3df);
        i &= w;
        i ^= i >> 5;
        if i < n {
            break;
        }
    }
    i.wrapping_add(hash) % n
}

/// Owen scrambling in base 2 of a 32-bit number whose bit 31 is the first
/// digit: every digit is flipped or not by a hash of the seed and the
/// digits above it (Owen 1995), as Laine and Karras's 2011 hash on the
/// bit-reversed number with Burley's 2020 constants. Applied to a sample
/// index instead, it is a nested uniform shuffle.
pub fn owen_scramble(mut v: u32, seed: u32) -> u32 {
    v = v.reverse_bits();
    v ^= v.wrapping_mul(0x3d20adea);
    v = v.wrapping_add(seed);
    v = v.wrapping_mul((seed >> 16) | 1);
    v ^= v.wrapping_mul(0x05526c56);
    v ^= v.wrapping_mul(0x53a22864);
    v.reverse_bits()
}

pub fn owen_scrambled_radical_inverse(base: u32, index: u32, hash: u64) -> f32 {
    let inv_base: f64 = 1.0 / (base as f64);
    let mut inv_base_m: f64 = 1.0; // base^-m after m digits
    let mut reversed: u64 = 0;
    let mut state: u64 = mix64(hash);
    let mut idx: u32 = index;

    // as long as a digit at this position still moves a float
    while 1.0f32 - (((base - 1) as f64 * inv_base_m) as f32) < 1.0f32 {
        let next: u32 = idx / base;
        let digit: u32 = idx - next * base;
        let hi: u32 = high32(state);
        let lo: u32 = low32(state);
        let a: u32 = 1 + ((hi as u64 * (base - 1) as u64) >> 32) as u32;
        let c: u32 = ((lo as u64 * base as u64) >> 32) as u32;
        let scrambled: u32 = (a * digit + c) % base;
        reversed = reversed * (base as u64) + (scrambled as u64);
        inv_base_m *= inv_base;
        idx = next;
        state = mix64(state ^ ((scrambled as u64) + 1));
    }

    let value: f32 = (inv_base_m * (reversed as f64)) as f32;
    return value.min(ONE_MINUS_EPSILON);
}

/// PCG32 (O'Neill 2014), as in pbrt-v4: a 64-bit linear congruential
/// state with a permuted 32-bit output, one stream per sequence index, and
/// a jump ahead by any number of steps in O(log n).
#[derive(Clone, Debug)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

const PCG32_MULT: u64 = 0x5851f42d4c957f2d;

impl Default for Pcg32 {
    fn default() -> Self {
        Self::new()
    }
}

impl Pcg32 {
    #[inline]
    pub const fn new() -> Self {
        Self {
            state: 0x853c49e6748fea9b,
            inc: 0xda3e39cb94b95bdb,
        }
    }

    pub fn set_sequence(&mut self, sequence: u64, seed: u64) {
        self.state = 0;
        self.inc = (sequence << 1) | 1;
        self.next();
        self.state = self.state.wrapping_add(seed);
        self.next();
    }

    /// The next 32 random bits.
    pub fn next(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(PCG32_MULT).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// The next random number as an f32 in [0, 1).
    pub fn uniform(&mut self) -> f32 {
        fixed_to_float(self.next())
    }

    /// Move the state on by delta steps (Brown 1994, "Random number
    /// generation with arbitrary strides").
    pub fn advance(&mut self, mut delta: u64) {
        let (mut cur_mult, mut cur_plus, mut acc_mult, mut acc_plus) =
            (PCG32_MULT, self.inc, 1u64, 0u64);
        while delta > 0 {
            if delta & 1 == 1 {
                acc_mult = acc_mult.wrapping_mul(cur_mult);
                acc_plus = acc_plus.wrapping_mul(cur_mult).wrapping_add(cur_plus);
            }
            cur_plus = cur_mult.wrapping_add(1).wrapping_mul(cur_plus);
            cur_mult = cur_mult.wrapping_mul(cur_mult);
            delta /= 2;
        }
        self.state = acc_mult.wrapping_mul(self.state).wrapping_add(acc_plus);
    }
}

/// The pixel and the seed pick a PCG32 stream, and sample `index` starts
/// 65536 steps per index into it, so a sample can use up to 65536 random
/// numbers before it would run into the next one's. independent draws
/// every dimension from it, stratified only the jitter inside a cell.
pub fn start_stream(rng: &mut Pcg32, x: i32, y: i32, index: i32, seed: u64) {
    rng.set_sequence(hash3(x as u32 as u64, y as u32 as u64, seed), seed);
    rng.advance((index as u64).wrapping_mul(65536));
}
