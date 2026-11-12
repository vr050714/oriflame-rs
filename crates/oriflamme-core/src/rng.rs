//! Small deterministic random number generator (SplitMix64).
//!
//! The rules engine and the bots only need shuffling and uniform picks, so a
//! self-contained generator keeps the crates dependency-free and makes every
//! game replayable from a seed.

/// Deterministic pseudo-random number generator.
#[derive(Clone, Debug)]
pub struct Rng64(u64);

impl Rng64 {
    /// Creates a generator from a seed. The same seed gives the same sequence.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Returns the next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ z.wrapping_shr(30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ z.wrapping_shr(27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ z.wrapping_shr(31)
    }

    /// Returns a value in `0..n`, or 0 when `n` is 0.
    pub fn below(&mut self, n: usize) -> usize {
        let n64 = u64::try_from(n).unwrap_or(u64::MAX);
        let r = self.next_u64().checked_rem(n64).unwrap_or(0);
        usize::try_from(r).unwrap_or(0)
    }

    /// Returns a value in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        // 53 random bits fit exactly in an f64 mantissa.
        let bits = self.next_u64().wrapping_shr(11);
        #[allow(clippy::cast_precision_loss)]
        let value = bits as f64 / 9_007_199_254_740_992.0;
        value
    }

    /// Shuffles a slice in place (Fisher-Yates).
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i.saturating_add(1));
            items.swap(i, j);
        }
    }

    /// Creates an independent generator seeded from this one.
    pub fn fork(&mut self) -> Self {
        Self(self.next_u64())
    }
}
