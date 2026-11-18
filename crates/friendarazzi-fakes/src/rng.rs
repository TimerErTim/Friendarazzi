use std::sync::Mutex;

use friendarazzi_core::ports::RngSource;
use rand::{Rng, SeedableRng, rngs::StdRng};

/// Deterministic RNG for repeatable tests.
pub struct FixedRng {
    inner: Mutex<StdRng>,
}

impl FixedRng {
    pub fn seeded(seed: u64) -> Self {
        Self {
            inner: Mutex::new(StdRng::seed_from_u64(seed)),
        }
    }
}

impl Default for FixedRng {
    fn default() -> Self {
        Self::seeded(0x5EED_F41E_DA4A_2210)
    }
}

impl RngSource for FixedRng {
    fn rng(&self) -> Box<dyn Rng + '_> {
        Box::new(self.inner.lock().unwrap())
    }
}
