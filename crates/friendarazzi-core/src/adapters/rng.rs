use rand::{rand_core::UnwrapErr, rngs::SysRng};

use crate::ports::RngSource;

/// Production RNG backed by the operating system CSPRNG (`SysRng`).
pub struct SystemRng;

impl RngSource for SystemRng {
    fn rng(&self) -> Box<dyn rand::Rng + '_> {
        Box::new(UnwrapErr(SysRng))
    }
}
