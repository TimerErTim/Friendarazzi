use rand::{Error as RandError, RngCore};

/// Pluggable randomness source so production uses `OsRng` and tests can pin a deterministic seed.
pub trait RngPort: Send + Sync {
    fn next_u32(&self) -> u32;
    fn next_u64(&self) -> u64;
    fn fill_bytes(&self, dest: &mut [u8]);
}

/// Adapts `&dyn RngPort` to `rand::RngCore` for domain operations.
pub struct PortRng<'a>(pub &'a dyn RngPort);

impl RngCore for PortRng<'_> {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest);
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), RandError> {
        self.0.fill_bytes(dest);
        Ok(())
    }
}
