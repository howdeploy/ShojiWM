//! Changing ShojiWM's own process environment.
//!
//! The embedded xwayland-satellite runs on its own thread and calls C libraries
//! (libwayland, xcb-util-cursor) that read the environment with `getenv`, which
//! races with a concurrent `setenv`. Every environment change in ShojiWM goes
//! through here and holds the satellite's `ENV_LOCK` for writing; satellite holds
//! it for reading around those C calls, none of which wait on this process.

use std::ffi::OsStr;
use std::sync::atomic::{AtomicU64, Ordering};

/// Bumped after every environment change, so [`EnvFlag`]s know to look again.
static ENV_GENERATION: AtomicU64 = AtomicU64::new(0);

fn env_write_guard() -> std::sync::RwLockWriteGuard<'static, ()> {
    satellite::ENV_LOCK
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// `std::env::set_var`, serialized with the embedded satellite's `getenv` calls.
pub fn set_var(key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) {
    let _guard = env_write_guard();
    // SAFETY: Rust's std serializes its own environment accesses, and the only
    // C `getenv` callers on other threads (the embedded satellite) hold
    // `ENV_LOCK` for reading, which `_guard` excludes.
    unsafe { std::env::set_var(key, value) };
    ENV_GENERATION.fetch_add(1, Ordering::Release);
}

/// `std::env::remove_var`, serialized like [`set_var`].
pub fn remove_var(key: impl AsRef<OsStr>) {
    let _guard = env_write_guard();
    // SAFETY: see `set_var`.
    unsafe { std::env::remove_var(key) };
    ENV_GENERATION.fetch_add(1, Ordering::Release);
}

/// Whether an environment variable is set, looked up once per environment change
/// instead of on every call. The debug switches sit on per-frame paths, and each
/// `std::env::var_os` takes the environment lock. Use through [`crate::env_flag!`].
pub struct EnvFlag {
    name: &'static str,
    /// `0` before the first lookup, else `(generation + 1) << 1 | is_set`.
    state: AtomicU64,
}

impl EnvFlag {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            state: AtomicU64::new(0),
        }
    }

    pub fn get(&self) -> bool {
        let generation = ENV_GENERATION.load(Ordering::Acquire);
        let state = self.state.load(Ordering::Relaxed);
        if state >> 1 == generation.wrapping_add(1) {
            return state & 1 == 1;
        }
        let is_set = std::env::var_os(self.name).is_some();
        self.state.store(
            (generation.wrapping_add(1) << 1) | u64::from(is_set),
            Ordering::Relaxed,
        );
        is_set
    }
}

/// `std::env::var_os(NAME).is_some()`, cached until the environment changes
/// through [`set_var`] / [`remove_var`].
#[macro_export]
macro_rules! env_flag {
    ($name:literal) => {{
        static FLAG: $crate::process_env::EnvFlag = $crate::process_env::EnvFlag::new($name);
        FLAG.get()
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn env_flag_sees_changes_made_through_process_env() {
        const NAME: &str = "SHOJI_ENV_FLAG_TEST_VARIABLE";
        let read = || crate::env_flag!("SHOJI_ENV_FLAG_TEST_VARIABLE");
        super::remove_var(NAME);
        assert!(!read());
        super::set_var(NAME, "1");
        assert!(read());
        super::remove_var(NAME);
        assert!(!read());
    }
}
