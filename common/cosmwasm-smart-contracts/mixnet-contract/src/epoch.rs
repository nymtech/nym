// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use network_defaults::{DEFAULT_EPOCH_LENGTH, DEFAULT_FIRST_EPOCH_START};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::time::Duration;
use time::OffsetDateTime;

/// Representation of rewarding epoch.
#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Epoch {
    id: i32,
    start: OffsetDateTime,
    length: Duration,
}

impl Epoch {
    /// Creates new epoch instance.
    pub const fn new(id: i32, start: OffsetDateTime, length: Duration) -> Self {
        Epoch { id, start, length }
    }

    /// Returns the next epoch.
    pub fn next_epoch(&self) -> Self {
        Epoch {
            id: self.id.saturating_add(1),
            start: self.end(),
            length: self.length,
        }
    }

    /// Returns the last epoch.
    pub fn previous_epoch(&self) -> Self {
        Epoch {
            id: self.id.saturating_sub(1),
            start: self.start - self.length,
            length: self.length,
        }
    }

    /// Determines whether the provided datetime is contained within the epoch
    ///
    /// # Arguments
    ///
    /// * `datetime`: specified datetime
    pub fn contains(&self, datetime: OffsetDateTime) -> bool {
        self.start <= datetime && datetime <= self.end()
    }

    /// Returns new instance of [Epoch] such that the provided datetime would be within
    /// its duration.
    ///
    /// # Arguments
    ///
    /// * `now`: current datetime
    pub fn current(&self, now: OffsetDateTime) -> Self {
        let mut candidate = self.clone();

        if now > self.start {
            loop {
                if candidate.contains(now) {
                    return candidate;
                }
                candidate = candidate.next_epoch();
            }
        } else {
            loop {
                if candidate.contains(now) {
                    return candidate;
                }
                candidate = candidate.previous_epoch();
            }
        }
    }

    pub const fn id(&self) -> i32 {
        self.id
    }

    /// Returns the starting datetime of this epoch.
    pub const fn start(&self) -> OffsetDateTime {
        self.start
    }

    /// Returns the length of this epoch.
    pub const fn length(&self) -> Duration {
        self.length
    }

    /// Returns the ending datetime of this epoch.
    pub fn end(&self) -> OffsetDateTime {
        self.start + self.length
    }

    /// Returns the unix timestamp of the start of this epoch.
    pub const fn start_unix_timestamp(&self) -> i64 {
        self.start().unix_timestamp()
    }

    /// Returns the unix timestamp of the end of this epoch.
    pub fn end_unix_timestamp(&self) -> i64 {
        self.end().unix_timestamp()
    }
}

impl Display for Epoch {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let length = self.length();
        let hours = length.as_secs_f32() / 3600.0;
        write!(
            f,
            "Epoch {}: {} - {} ({:.1} hours)",
            self.id,
            self.start(),
            self.end(),
            hours
        )
    }
}

impl Default for Epoch {
    fn default() -> Self {
        Epoch {
            id: 0,
            start: DEFAULT_FIRST_EPOCH_START,
            length: DEFAULT_EPOCH_LENGTH,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previous_epoch() {
        let epoch = Epoch {
            id: 1,
            start: time::macros::datetime!(2021-08-23 12:00 UTC),
            length: Duration::from_secs(24 * 60 * 60),
        };
        let expected = Epoch {
            id: 0,
            start: time::macros::datetime!(2021-08-22 12:00 UTC),
            length: Duration::from_secs(24 * 60 * 60),
        };

        assert_eq!(expected, epoch.previous_epoch())
    }

    #[test]
    fn next_epoch() {
        let epoch = Epoch {
            id: 0,
            start: time::macros::datetime!(2021-08-23 12:00 UTC),
            length: Duration::from_secs(24 * 60 * 60),
        };
        let expected = Epoch {
            id: 1,
            start: time::macros::datetime!(2021-08-24 12:00 UTC),
            length: Duration::from_secs(24 * 60 * 60),
        };

        assert_eq!(expected, epoch.next_epoch())
    }

    #[test]
    fn checking_for_datetime_inclusion() {
        let epoch = Epoch {
            id: 0,
            start: time::macros::datetime!(2021-08-23 12:00 UTC),
            length: Duration::from_secs(24 * 60 * 60),
        };

        // it must contain its own boundaries
        assert!(epoch.contains(epoch.start));
        assert!(epoch.contains(epoch.end()));

        let in_the_midle = epoch.start + Duration::from_secs(epoch.length.as_secs() / 2);
        assert!(epoch.contains(in_the_midle));

        assert!(!epoch.contains(epoch.next_epoch().end()));
        assert!(!epoch.contains(epoch.previous_epoch().start()));
    }

    #[test]
    fn determining_current_epoch() {
        let first_epoch = Epoch {
            id: 0,
            start: time::macros::datetime!(2021-08-23 12:00 UTC),
            length: Duration::from_secs(24 * 60 * 60),
        };

        // epoch just before
        let fake_now = first_epoch.start - Duration::from_secs(123);
        assert_eq!(first_epoch.previous_epoch(), first_epoch.current(fake_now));

        // this epoch (start boundary)
        assert_eq!(first_epoch, first_epoch.current(first_epoch.start));

        // this epoch (in the middle)
        let fake_now = first_epoch.start + Duration::from_secs(123);
        assert_eq!(first_epoch, first_epoch.current(fake_now));

        // this epoch (end boundary)
        assert_eq!(first_epoch, first_epoch.current(first_epoch.end()));

        // next epoch
        let fake_now = first_epoch.end() + Duration::from_secs(123);
        assert_eq!(first_epoch.next_epoch(), first_epoch.current(fake_now));

        // few epochs in the past
        let fake_now =
            first_epoch.start() - first_epoch.length - first_epoch.length - first_epoch.length;
        assert_eq!(
            first_epoch
                .previous_epoch()
                .previous_epoch()
                .previous_epoch(),
            first_epoch.current(fake_now)
        );

        // few epochs in the future
        let fake_now =
            first_epoch.end() + first_epoch.length + first_epoch.length + first_epoch.length;
        assert_eq!(
            first_epoch.next_epoch().next_epoch().next_epoch(),
            first_epoch.current(fake_now)
        );
    }
}
