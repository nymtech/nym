// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::fmt::{Display, Formatter};
use std::time::Duration;
use time::OffsetDateTime;

// internally, since version 0.3.6, time uses deserialize_any for deserialization, which can't be handled
// by serde wasm. We could just downgrade to 0.3.5 and call it a day, but then it would break
// when we decided to upgrade it at some point in the future. And then it would have been more problematic
// to fix it, since the data would have already been stored inside the contract.
// Hence, an explicit workaround to use string representation of Rfc3339-formatted datetime.
pub(crate) mod string_rfc3339_offset_date_time {
    use serde::de::Visitor;
    use serde::ser::Error;
    use serde::{Deserializer, Serialize, Serializer};
    use std::fmt::Formatter;
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;

    struct Rfc3339OffsetDateTimeVisitor;

    impl<'de> Visitor<'de> for Rfc3339OffsetDateTimeVisitor {
        type Value = OffsetDateTime;

        fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("an rfc3339 `OffsetDateTime`")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            OffsetDateTime::parse(value, &Rfc3339).map_err(E::custom)
        }
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(Rfc3339OffsetDateTimeVisitor)
    }

    pub(crate) fn serialize<S>(datetime: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        datetime
            .format(&Rfc3339)
            .map_err(S::Error::custom)?
            .serialize(serializer)
    }
}

/// Representation of rewarding interval.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Interval {
    id: u32,
    #[serde(with = "string_rfc3339_offset_date_time")]
    start: OffsetDateTime,
    length: Duration,
}

impl Interval {
    /// Initialize epoch in the contract with default values.
    pub fn init_epoch() -> Self {
        Interval { id: 0, start: OffsetDateTime::now_utc(), length: Duration::from_secs(3600) }
    }

    /// Returns the next interval.
    #[must_use]
    pub fn next(&self) -> Self {
        Interval {
            id: self.id + 1,
            start: self.end(),
            length: self.length,
        }
    }

    /// Returns the last interval.
    pub fn previous(&self) -> Option<Self> {
        if self.id > 0 {
            Some(Interval {
                id: self.id - 1,
                start: self.start - self.length,
                length: self.length,
            })
        } else {
            None
        }
    }

    /// Determines whether the provided datetime is contained within the interval
    ///
    /// # Arguments
    ///
    /// * `datetime`: specified datetime
    pub fn contains(&self, datetime: OffsetDateTime) -> bool {
        self.start <= datetime && datetime <= self.end()
    }

    /// Determines whether the provided unix timestamp is contained within the interval
    ///
    /// # Arguments
    ///
    /// * `timestamp`: specified timestamp
    pub fn contains_timestamp(&self, timestamp: i64) -> bool {
        self.start_unix_timestamp() <= timestamp && timestamp <= self.end_unix_timestamp()
    }

    /// Returns new instance of [Interval] such that the provided datetime would be within
    /// its duration.
    ///
    /// # Arguments
    ///
    /// * `now`: current datetime
    pub fn current(&self, now: OffsetDateTime) -> Option<Self> {
        let mut candidate = *self;

        if now > self.start {
            loop {
                if candidate.contains(now) {
                    return Some(candidate);
                }
                candidate = candidate.next();
            }
        } else {
            loop {
                if candidate.contains(now) {
                    return Some(candidate);
                }
                candidate = candidate.previous()?;
            }
        }
    }

    /// Returns new instance of [Interval] such that the provided unix timestamp would be within
    /// its duration.
    ///
    /// # Arguments
    ///
    /// * `now_unix`: current unix time
    pub fn current_with_timestamp(&self, now_unix: i64) -> Option<Self> {
        let mut candidate = *self;

        if now_unix > self.start_unix_timestamp() {
            loop {
                if candidate.contains_timestamp(now_unix) {
                    return Some(candidate);
                }
                candidate = candidate.next();
            }
        } else {
            loop {
                if candidate.contains_timestamp(now_unix) {
                    return Some(candidate);
                }
                candidate = candidate.previous()?;
            }
        }
    }

    /// Checks whether this interval has already finished
    ///
    /// # Arguments
    ///
    /// * `now`: current datetime
    pub fn has_elapsed(&self, now: OffsetDateTime) -> bool {
        self.end() < now
    }

    /// Returns id of this interval
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// Determines amount of time left until this interval finishes.
    ///
    /// # Arguments
    ///
    /// * `now`: current datetime
    pub fn until_end(&self, now: OffsetDateTime) -> Option<Duration> {
        let remaining = self.end() - now;
        if remaining.is_negative() {
            None
        } else {
            remaining.try_into().ok()
        }
    }

    /// Returns the starting datetime of this interval.
    pub const fn start(&self) -> OffsetDateTime {
        self.start
    }

    /// Returns the length of this interval.
    pub const fn length(&self) -> Duration {
        self.length
    }

    /// Returns the ending datetime of this interval.
    pub fn end(&self) -> OffsetDateTime {
        self.start + self.length
    }

    /// Returns the unix timestamp of the start of this interval.
    pub const fn start_unix_timestamp(&self) -> i64 {
        self.start().unix_timestamp()
    }

    /// Returns the unix timestamp of the end of this interval.
    pub fn end_unix_timestamp(&self) -> i64 {
        self.end().unix_timestamp()
    }
}

impl Display for Interval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let length = self.length().as_secs();
        let full_hours = length / 3600;
        let rem = length % 3600;
        write!(
            f,
            "Interval {}: {} - {} ({}h {}s)",
            self.id,
            self.start(),
            self.end(),
            full_hours,
            rem
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previous() {
        let interval = Interval {
            id: 1,
            start: time::macros::datetime!(2021-08-23 12:00 UTC),
            length: Duration::from_secs(24 * 60 * 60),
        };
        let expected = Interval {
            id: 0,
            start: time::macros::datetime!(2021-08-22 12:00 UTC),
            length: Duration::from_secs(24 * 60 * 60),
        };
        assert_eq!(expected, interval.previous().unwrap());

        let genesis_interval = Interval {
            id: 0,
            start: time::macros::datetime!(2021-08-23 12:00 UTC),
            length: Duration::from_secs(24 * 60 * 60),
        };
        assert!(genesis_interval.previous().is_none());
    }

    #[test]
    fn next() {
        let interval = Interval {
            id: 0,
            start: time::macros::datetime!(2021-08-23 12:00 UTC),
            length: Duration::from_secs(24 * 60 * 60),
        };
        let expected = Interval {
            id: 1,
            start: time::macros::datetime!(2021-08-24 12:00 UTC),
            length: Duration::from_secs(24 * 60 * 60),
        };

        assert_eq!(expected, interval.next())
    }

    #[test]
    fn checking_for_datetime_inclusion() {
        let interval = Interval {
            id: 100,
            start: time::macros::datetime!(2021-08-23 12:00 UTC),
            length: Duration::from_secs(24 * 60 * 60),
        };

        // it must contain its own boundaries
        assert!(interval.contains(interval.start));
        assert!(interval.contains(interval.end()));

        let in_the_midle = interval.start + Duration::from_secs(interval.length.as_secs() / 2);
        assert!(interval.contains(in_the_midle));

        assert!(!interval.contains(interval.next().end()));
        assert!(!interval.contains(interval.previous().unwrap().start()));
    }

    #[test]
    fn determining_current_interval() {
        let first_interval = Interval {
            id: 100,
            start: time::macros::datetime!(2021-08-23 12:00 UTC),
            length: Duration::from_secs(24 * 60 * 60),
        };

        // interval just before
        let fake_now = first_interval.start - Duration::from_secs(123);
        assert_eq!(first_interval.previous(), first_interval.current(fake_now));

        // this interval (start boundary)
        assert_eq!(
            first_interval,
            first_interval.current(first_interval.start).unwrap()
        );

        // this interval (in the middle)
        let fake_now = first_interval.start + Duration::from_secs(123);
        assert_eq!(first_interval, first_interval.current(fake_now).unwrap());

        // this interval (end boundary)
        assert_eq!(
            first_interval,
            first_interval.current(first_interval.end()).unwrap()
        );

        // next interval
        let fake_now = first_interval.end() + Duration::from_secs(123);
        assert_eq!(
            first_interval.next(),
            first_interval.current(fake_now).unwrap()
        );

        // few intervals in the past
        let fake_now = first_interval.start()
            - first_interval.length
            - first_interval.length
            - first_interval.length;
        assert_eq!(
            first_interval
                .previous()
                .unwrap()
                .previous()
                .unwrap()
                .previous()
                .unwrap(),
            first_interval.current(fake_now).unwrap()
        );

        // few intervals in the future
        let fake_now = first_interval.end()
            + first_interval.length
            + first_interval.length
            + first_interval.length;
        assert_eq!(
            first_interval.next().next().next(),
            first_interval.current(fake_now).unwrap()
        );
    }
}
