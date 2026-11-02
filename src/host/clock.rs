// SPDX-License-Identifier: MIT

//! Host clock capability.
//!
//! This is the only place MiniPython reads the host wall clock or decides
//! the local time zone, and the only monotonic clock read on the worker side
//! ([`Stopwatch`]). `datetime.now()`, `date.today()`, `utcnow()`,
//! `fromtimestamp()`, naive `datetime.timestamp()`, and `astimezone()` with
//! no argument all go through it.
//!
//! Sandbox contract: the clock is the real wall clock unless the embedder
//! pins it, and the local time zone is always UTC. `TZ` and
//! `/etc/localtime` are never read.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Offset of the sandbox's local time zone from UTC, in seconds.
pub const LOCAL_UTC_OFFSET_SECONDS: i64 = 0;

/// Name of the sandbox's local time zone (`datetime.astimezone().tzname()`).
pub const LOCAL_TIMEZONE_NAME: &str = "UTC";

/// Where the sandbox reads the current time from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HostClock {
    /// The host's real wall clock. This is the default for the CLI,
    /// `Sandbox`, and the in-process APIs.
    #[default]
    System,
    /// Every reading returns this instant, in microseconds since the Unix
    /// epoch, so runs are deterministic.
    Fixed { unix_micros: i64 },
}

impl HostClock {
    /// A clock pinned to `unix_micros` microseconds since the Unix epoch.
    pub fn fixed_unix_micros(unix_micros: i64) -> Self {
        Self::Fixed { unix_micros }
    }

    /// A clock pinned to `instant`.
    pub fn fixed(instant: SystemTime) -> Self {
        Self::Fixed {
            unix_micros: system_time_to_unix_micros(instant),
        }
    }

    /// The current time in microseconds since the Unix epoch.
    pub fn now_unix_micros(&self) -> i64 {
        match self {
            Self::System => system_time_to_unix_micros(SystemTime::now()),
            Self::Fixed { unix_micros } => *unix_micros,
        }
    }
}

/// Offset of local time from UTC at the given instant, in seconds.
pub(crate) fn local_utc_offset_seconds(_unix_seconds: i64) -> i64 {
    LOCAL_UTC_OFFSET_SECONDS
}

/// Measures elapsed wall time for `ExecutionUsage::wall_time_micros`. Python
/// code cannot read it.
pub(crate) struct Stopwatch(Instant);

impl Stopwatch {
    pub(crate) fn start() -> Self {
        Self(Instant::now())
    }

    pub(crate) fn elapsed_micros(&self) -> u64 {
        u64::try_from(self.0.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}

fn system_time_to_unix_micros(instant: SystemTime) -> i64 {
    match instant.duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_micros()).unwrap_or(i64::MAX),
        Err(before) => {
            let micros = before.duration().as_micros();
            i64::try_from(micros).map_or(i64::MIN, |micros| -micros)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn fixed_clock_returns_its_instant() {
        let clock = HostClock::fixed(UNIX_EPOCH + Duration::from_micros(1_234_567));
        assert_eq!(clock.now_unix_micros(), 1_234_567);
        let before = HostClock::fixed(UNIX_EPOCH - Duration::from_micros(5));
        assert_eq!(before.now_unix_micros(), -5);
    }

    #[test]
    fn system_clock_is_after_2020() {
        assert!(HostClock::System.now_unix_micros() > 1_577_836_800_000_000);
    }
}
