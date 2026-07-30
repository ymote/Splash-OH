//! Dart's `Random`, so a seeded sequence can be replayed here.
//!
//! Wonderous places its clouds with `rndSeed = <per-wonder seed>` and then a
//! run of `rnd.getDouble` / `rnd.getBool` calls. Reproducing where the clouds
//! land therefore means reproducing Dart's generator exactly, not merely
//! something with the same distribution.
//!
//! Transcribed from `sdk/lib/_internal/vm/lib/math_patch.dart` (`_Random`) and
//! from the `rnd` package's `getDouble`/`getBool`. Dart's algorithm is not part
//! of its published API contract, so this tracks a specific implementation
//! rather than a promise — but it is the implementation the app is built
//! against, which is what matters for matching the app.
//!
//! Dart's integers are 64-bit and wrap, and `>>>` is a logical shift on the
//! 64-bit pattern, so `u64` with wrapping arithmetic is an exact match.

pub struct DartRandom {
    state: u64,
}

impl DartRandom {
    /// `Random(seed)`: scramble the seed, then crank four times.
    pub fn new(seed: i64) -> Self {
        let mut r = DartRandom {
            state: Self::setup_seed(seed as u64),
        };
        for _ in 0..4 {
            r.next_state();
        }
        r
    }

    /// `_setupSeed`, verbatim.
    fn setup_seed(mut n: u64) -> u64 {
        n = (!n).wrapping_add(n << 21);
        n ^= n >> 24;
        n = n.wrapping_mul(265);
        n ^= n >> 14;
        n = n.wrapping_mul(21);
        n ^= n >> 28;
        n = n.wrapping_add(n << 31);
        if n == 0 {
            0x5a17
        } else {
            n
        }
    }

    /// Multiply-with-carry, base 2^32, A from Numerical Recipes 3rd ed. p.348.
    fn next_state(&mut self) {
        const A: u64 = 0xffff_da61;
        let lo = self.state & 0xFFFF_FFFF;
        let hi = self.state >> 32;
        self.state = A.wrapping_mul(lo).wrapping_add(hi);
    }

    /// `nextInt` for a power-of-two bound, which is the only shape
    /// `nextDouble` asks for.
    fn next_bits(&mut self, bits: u32) -> u64 {
        self.next_state();
        (self.state & 0xFFFF_FFFF) & ((1u64 << bits) - 1)
    }

    pub fn next_double(&mut self) -> f64 {
        let hi = self.next_bits(26) as f64;
        let lo = self.next_bits(27) as f64;
        (hi * (1u64 << 27) as f64 + lo) / (1u64 << 53) as f64
    }

    /// `rnd.getDouble(min, max)`.
    pub fn get_double(&mut self, min: f64, max: f64) -> f64 {
        min + self.next_double() * (max - min)
    }

    /// `rnd.getBool()` — the package's default chance is .5.
    pub fn get_bool(&mut self) -> bool {
        self.next_double() < 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::DartRandom;

    /// The generator has to behave like a generator before it is worth asking
    /// whether it is Dart's: uniform over [0, 1), and the same every run.
    #[test]
    fn uniform_and_deterministic() {
        let mut r = DartRandom::new(1);
        let mut sum = 0.0;
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for _ in 0..20_000 {
            let v = r.next_double();
            assert!((0.0..1.0).contains(&v), "out of range: {v}");
            sum += v;
            lo = lo.min(v);
            hi = hi.max(v);
        }
        let mean = sum / 20_000.0;
        assert!((mean - 0.5).abs() < 0.02, "mean {mean} is not uniform");
        assert!(lo < 0.001 && hi > 0.999, "range {lo}..{hi} is too narrow");

        let a: Vec<f64> = (0..5).map(|_| DartRandom::new(78).next_double()).collect();
        assert!(a.windows(2).all(|w| w[0] == w[1]), "seed 78 is not stable");
    }

    /// The first five doubles for seed 78 — Christ the Redeemer's cloud seed.
    ///
    /// These are not from a Dart run; there is no Dart here. They come from a
    /// second, independent transcription of the same published algorithm, so
    /// what this pins is that the Rust below says the same thing that source
    /// does. A slip in a shift width or in the MWC constant changes them.
    #[test]
    fn matches_reference_sequence() {
        let mut r = DartRandom::new(78);
        let got: Vec<f64> = (0..5).map(|_| r.next_double()).collect();
        let want = [
            0.5048283223826905,
            0.7766267639990284,
            0.7292461573942316,
            0.8174941250096173,
            0.8085785783220495,
        ];
        for (g, w) in got.iter().zip(want.iter()) {
            assert!((g - w).abs() < 1e-12, "got {got:?}, want {want:?}");
        }
    }
}
