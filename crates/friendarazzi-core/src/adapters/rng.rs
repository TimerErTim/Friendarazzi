use rand::{RngCore, rngs::OsRng};
use tracing::trace;

use crate::ports::RngPort;

/// Production RNG backed by the operating system CSPRNG (`OsRng`).
pub struct SystemRng;

impl RngPort for SystemRng {
    fn next_u32(&self) -> u32 {
        trace!("system rng next_u32");
        OsRng.next_u32()
    }

    fn next_u64(&self) -> u64 {
        trace!("system rng next_u64");
        OsRng.next_u64()
    }

    fn fill_bytes(&self, dest: &mut [u8]) {
        trace!(len = dest.len(), "system rng fill_bytes");
        OsRng.fill_bytes(dest);
    }
}
