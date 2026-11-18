use rand::Rng;

/// Pluggable randomness source so production uses `SysRng` and tests can pin a deterministic seed.
pub trait RngSource: Send + Sync {
    fn rng(&self) -> Box<dyn Rng + '_>;
}
