use std::sync::Mutex;

use friendarazzi_core::ports::RngPort;
use rand::{RngCore, SeedableRng, rngs::StdRng};

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

impl RngPort for FixedRng {
    fn next_u32(&self) -> u32 {
        self.inner.lock().unwrap().next_u32()
    }

    fn next_u64(&self) -> u64 {
        self.inner.lock().unwrap().next_u64()
    }

    fn fill_bytes(&self, dest: &mut [u8]) {
        self.inner.lock().unwrap().fill_bytes(dest);
    }
}
