//! Changing ShojiWM's own process environment.
//!
//! The embedded xwayland-satellite runs on its own thread and calls C libraries
//! (libwayland, xcb-util-cursor) that read the environment with `getenv`, which
//! races with a concurrent `setenv`. Every environment change in ShojiWM goes
//! through here and holds the satellite's `ENV_LOCK` for writing; satellite holds
//! it for reading around those C calls, none of which wait on this process.

use std::ffi::OsStr;

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
}

/// `std::env::remove_var`, serialized like [`set_var`].
pub fn remove_var(key: impl AsRef<OsStr>) {
    let _guard = env_write_guard();
    // SAFETY: see `set_var`.
    unsafe { std::env::remove_var(key) };
}
