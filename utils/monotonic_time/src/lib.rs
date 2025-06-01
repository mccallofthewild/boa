//! This crate provides utilities for monotonic time manipulation.

/*───────────────────────────  global time slot  ───────────────────────────*/

use std::{cell::Cell, time::Duration};

/// Simple wrapper around `Cell<T>` that we mark `Sync` **unsafely**.
/// In `wasm32‑unknown‑unknown` CosmWasm the VM is single‑threaded, so this is
/// safe.  On native builds it behaves like a `static mut` guarded by the user.
struct Global<T>(Cell<T>);
//  SAFETY: CosmWasm executes contract code on a single thread.  The only
//  possible race is in tests or native builds, where the developer must ensure
//  they do not call entry‑points concurrently.
unsafe impl<T> Sync for Global<T> {}

static GLOBAL_TIME_NS: Global<Option<u128>> = Global(Cell::new(None));
static GLOBAL_BUMP: Global<u64> = Global(Cell::new(0));

/// Publish the current block‑time (nanoseconds since epoch).
#[inline]
pub fn set_time_nanos(nanos: u128) {
    GLOBAL_TIME_NS.0.set(Some(nanos));
    GLOBAL_BUMP.0.set(0);
}

/// Clear the global slot (call in all normal and error return paths).
#[inline]
pub fn clear_time() {
    GLOBAL_TIME_NS.0.set(None);
    GLOBAL_BUMP.0.set(0);
}

/// Internal: return a [`Duration`] representing the next monotone instant.
#[inline]
pub fn next_duration() -> Duration {
    let base = GLOBAL_TIME_NS
        .0
        .get()
        .expect("GLOBAL_TIME_NS not initialised – call set_time_nanos() first");
    let bump = GLOBAL_BUMP.0.get();
    GLOBAL_BUMP.0.set(bump.wrapping_add(1));

    let total = base + bump as u128; // total nanoseconds since epoch
    let secs = (total / 1_000_000_000) as u64;
    let nanos = (total % 1_000_000_000) as u32;
    Duration::new(secs, nanos)
}
