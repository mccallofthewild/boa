//! Clock related types and functions – **no std::time::SystemTime, no atomics**
//!
//! * Call [`set_time_nanos`] **once** at the very top of every external
//!   entry‑point (instantiate/execute/query/…) with the block‑time expressed in
//!   nanoseconds since Unix epoch.
//! * Any deep code can then obtain a strictly‑monotonic instant via
//!   [`StdClock::now`].  Each successive `now()` within the same entry‑point
//!   is guaranteed to be **≥** the previous one (base time + bump [ns]).
//! * Call [`clear_time`] before returning from the entry‑point to avoid
//!   accidental leakage when the VM reuses the same Wasm instance.
//!
//! Public surface (structs / traits) remains unchanged, ensuring full
//! compatibility for upstream code, but all host‑clock and threading
//! dependencies are removed.

#![allow(clippy::missing_inline_in_public_items)]

use core::{cell::Cell, time::Duration};
use monotonic_time::next_duration;

pub use monotonic_time::{clear_time, set_time_nanos};

/*────────────────────────────  JsInstant  ────────────────────────────────*/

/// A monotonic instant in time, in the Boa engine (nanosecond resolution).
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct JsInstant {
    /// Duration since the Unix epoch.
    inner: Duration,
}

impl JsInstant {
    /// Creates a new `JsInstant` from the given seconds / nanoseconds pair.
    #[must_use]
    pub fn new(secs: u64, nanos: u32) -> Self {
        let inner = Duration::new(secs, nanos);
        Self::new_unchecked(inner)
    }

    /// Creates a new `JsInstant` from an unchecked [`Duration`].
    #[must_use]
    fn new_unchecked(inner: Duration) -> Self {
        Self { inner }
    }

    /// Returns milliseconds since epoch.
    #[must_use]
    pub fn millis_since_epoch(&self) -> u64 {
        self.inner.as_millis() as u64
    }

    /// Returns nanoseconds since epoch.
    #[must_use]
    pub fn nanos_since_epoch(&self) -> u128 {
        self.inner.as_nanos()
    }
}

/*────────────────────────────  JsDuration  ───────────────────────────────*/

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct JsDuration {
    inner: Duration,
}

impl JsDuration {
    /// Creates a new `JsDuration` from the given number of milliseconds.
    #[must_use]
    pub fn from_millis(millis: u64) -> Self {
        Self {
            inner: Duration::from_millis(millis),
        }
    }

    #[must_use]
    pub fn as_millis(&self) -> u64 {
        self.inner.as_millis() as u64
    }
    #[must_use]
    pub fn as_secs(&self) -> u64 {
        self.inner.as_secs()
    }
    #[must_use]
    pub fn as_nanos(&self) -> u128 {
        self.inner.as_nanos()
    }
}

impl From<Duration> for JsDuration {
    fn from(d: Duration) -> Self {
        Self { inner: d }
    }
}
impl From<JsDuration> for Duration {
    fn from(d: JsDuration) -> Self {
        d.inner
    }
}

/*───────────────────────  duration/instant arithmetic  ───────────────────*/

macro_rules! impl_duration_ops {
    ($($trait:ident $fn:ident),*) => {
        $(
            impl core::ops::$trait for JsDuration {
                type Output = JsDuration;
                #[inline]
                fn $fn(self, rhs: JsDuration) -> Self::Output {
                    JsDuration { inner: core::ops::$trait::$fn(self.inner, rhs.inner) }
                }
            }
            impl core::ops::$trait<JsDuration> for JsInstant {
                type Output = JsInstant;
                #[inline]
                fn $fn(self, rhs: JsDuration) -> Self::Output {
                    JsInstant { inner: core::ops::$trait::$fn(self.inner, rhs.inner) }
                }
            }
        )*
    };
}
impl_duration_ops!(Add add, Sub sub);

impl core::ops::Sub for JsInstant {
    type Output = JsDuration;
    #[inline]
    fn sub(self, rhs: JsInstant) -> Self::Output {
        JsDuration {
            inner: self.inner - rhs.inner,
        }
    }
}

/*──────────────────────────────  Clock  ─────────────────────────────────*/

pub trait Clock {
    fn now(&self) -> JsInstant;
}

/// `StdClock` now reads from the deterministic global time slot.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdClock;
impl Clock for StdClock {
    fn now(&self) -> JsInstant {
        JsInstant::new_unchecked(next_duration())
    }
}

/// A fixed‑time clock, useful for unit tests.
#[derive(Debug, Clone, Default)]
pub struct FixedClock(core::cell::RefCell<u64>);
impl FixedClock {
    #[must_use]
    pub fn from_millis(millis: u64) -> Self {
        Self(core::cell::RefCell::new(millis))
    }
    pub fn forward(&self, millis: u64) {
        *self.0.borrow_mut() += millis;
    }
}
impl Clock for FixedClock {
    fn now(&self) -> JsInstant {
        let millis = *self.0.borrow();
        JsInstant::new_unchecked(Duration::new(
            millis / 1_000,
            ((millis % 1_000) * 1_000_000) as u32,
        ))
    }
}

/*──────────────────────────────  tests  ─────────────────────────────────*/

#[cfg(test)]
mod tests {
    use monotonic_time::{clear_time, set_time_nanos};

    use super::*;

    #[test]
    fn monotone() {
        set_time_nanos(1_700_000_000_000_000_000); // arbitrary epoch
        let clk = StdClock;
        let a = clk.now();
        let b = clk.now();
        assert!(b > a);
        assert_eq!(b.nanos_since_epoch() - a.nanos_since_epoch(), 1);
        clear_time();
    }
}
