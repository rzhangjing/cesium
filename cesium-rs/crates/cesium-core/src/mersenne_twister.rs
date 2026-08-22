//! Ported from the npm package `mersenne-twister` (imported by
//! `packages/engine/Source/Core/Math.js` line 1).
//!
//! DEVIATION: CesiumJS imports the third-party npm package
//! `mersenne-twister`; there is no `Source/Core/MersenneTwister.js` in the
//! upstream repository. This is a faithful re-implementation of that
//! package's MT19937 generator (Sean McCullough's port of the reference
//! C implementation) so that `next_random_number` is reproducible from a
//! seed exactly like the JS version. Registered in `docs/deviations.md`.

use std::time::{SystemTime, UNIX_EPOCH};

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908_b0df;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7fff_ffff;

/// JS `ToUint32` conversion (bitwise operators coerce via Int32/Uint32).
fn to_uint32(x: f64) -> u32 {
    if x.is_nan() || x.is_infinite() {
        return 0;
    }
    let x = x.trunc();
    let mut m = x % 4_294_967_296.0;
    if m < 0.0 {
        m += 4_294_967_296.0;
    }
    m as u32
}

pub(crate) struct MersenneTwister {
    mt: [u32; N],
    mti: usize,
}

impl MersenneTwister {
    /// Mirrors `new MersenneTwister(seed)`: a missing seed falls back to
    /// `new Date().getTime()` in the JS original.
    pub(crate) fn new(seed: Option<f64>) -> Self {
        let seed = seed.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0)
        });
        let mut twister = MersenneTwister {
            mt: [0; N],
            mti: N + 1,
        };
        twister.init_genrand(seed);
        twister
    }

    fn init_genrand(&mut self, s: f64) {
        self.mt[0] = to_uint32(s);
        for mti in 1..N {
            let prev = self.mt[mti - 1] ^ (self.mt[mti - 1] >> 30);
            // JS splits the multiplication into 16-bit halves; this is
            // bit-identical to `(prev * 1812433253 + mti) mod 2^32`.
            self.mt[mti] = prev
                .wrapping_mul(1_812_433_253)
                .wrapping_add(mti as u32);
            self.mti = mti;
        }
        self.mti = N;
    }

    pub(crate) fn genrand_int32(&mut self) -> u32 {
        let mut y: u32;
        let mag01: [u32; 2] = [0x0, MATRIX_A];

        if self.mti >= N {
            // generate N words at one time
            if self.mti == N + 1 {
                self.init_genrand(5489.0);
            }

            for kk in 0..(N - M) {
                y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] = self.mt[kk + M] ^ (y >> 1) ^ mag01[(y & 0x1) as usize];
            }
            for kk in (N - M)..(N - 1) {
                y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] = self.mt[kk + M - N] ^ (y >> 1) ^ mag01[(y & 0x1) as usize];
            }
            y = (self.mt[N - 1] & UPPER_MASK) | (self.mt[0] & LOWER_MASK);
            self.mt[N - 1] = self.mt[M - 1] ^ (y >> 1) ^ mag01[(y & 0x1) as usize];
            self.mti = 0;
        }

        y = self.mt[self.mti];
        self.mti += 1;

        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;

        y
    }

    /// `random()` in the npm package: `[0.0, 1.0)` from 32-bit precision.
    pub(crate) fn random(&mut self) -> f64 {
        self.genrand_int32() as f64 * (1.0 / 4_294_967_296.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference MT19937 outputs for seed 5489 (the standard
    /// `init_genrand(5489)` test vector shared by all reference ports).
    #[test]
    fn genrand_int32_matches_reference_vectors() {
        let mut twister = MersenneTwister::new(Some(5489.0));
        let expected: [u32; 5] = [
            3499211612,
            581869302,
            3890346734,
            3586334585,
            545404204,
        ];
        for want in expected {
            assert_eq!(twister.genrand_int32(), want);
        }
    }

    /// Seed coercion mirrors `s >>> 0` in the JS original.
    #[test]
    fn seed_is_coerced_like_js_uint32() {
        let mut a = MersenneTwister::new(Some(5489.0));
        let mut b = MersenneTwister::new(Some(5489.9)); // truncated
        assert_eq!(a.genrand_int32(), b.genrand_int32());
    }

    #[test]
    fn random_is_in_range() {
        let mut twister = MersenneTwister::new(Some(12345.0));
        for _ in 0..1000 {
            let value = twister.random();
            assert!(value >= 0.0 && value < 1.0);
        }
    }
}
