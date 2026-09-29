//! Hashing for the per-frame cache signatures (effect captures, backdrops, scene
//! contributors).
//!
//! These run for every effect on every frame, so they use FxHash rather than the
//! SipHash behind `DefaultHasher`, and feed `Debug` output to the hasher without
//! building a `String` first. A signature is only ever compared with one built by
//! the same code, so neither choice has to match anything else.

use std::fmt::{self, Debug, Write};
use std::hash::Hasher;

pub type SignatureHasher = rustc_hash::FxHasher;

/// Hashes `value`'s `Debug` output, for values without a `Hash` impl (floats,
/// effect descriptions). Equal output always hashes equally.
pub fn hash_debug<H: Hasher>(hasher: &mut H, value: &impl Debug) {
    struct HashWriter<'a, H>(&'a mut H);

    impl<H: Hasher> Write for HashWriter<'_, H> {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            self.0.write(text.as_bytes());
            Ok(())
        }
    }

    let _ = write!(HashWriter(hasher), "{value:?}");
    // Terminates the value the way `str`'s `Hash` does, so neighbouring values
    // cannot run into each other.
    hasher.write_u8(0xff);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signature(value: &impl Debug) -> u64 {
        let mut hasher = SignatureHasher::default();
        hash_debug(&mut hasher, value);
        hasher.finish()
    }

    #[test]
    fn debug_hash_follows_the_debug_output() {
        assert_eq!(signature(&(1.5f32, "a")), signature(&(1.5f32, "a")));
        assert_ne!(signature(&(1.5f32, "a")), signature(&(1.25f32, "a")));
        assert_ne!(signature(&("ab", "c")), signature(&("a", "bc")));
    }
}
