// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::pending_events::{PendingEpochEvent, PendingIntervalEvent};
use crate::{EpochId, IntervalId};
use cosmwasm_std::Env;
use schemars::gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Interval {
    id: IntervalId,
    epochs_in_interval: u32,

    #[serde(with = "string_rfc3339_offset_date_time")]
    current_epoch_start: OffsetDateTime,
    current_epoch_id: EpochId,
    epoch_length: Duration,
    total_elapsed_epochs: EpochId,
}

impl JsonSchema for Interval {
    fn schema_name() -> String {
        "Interval".to_owned()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        let mut schema_object = SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            ..SchemaObject::default()
        };

        let object_validation = schema_object.object();
        object_validation
            .properties
            .insert("id".to_owned(), gen.subschema_for::<IntervalId>());
        object_validation.required.insert("id".to_owned());

        object_validation
            .properties
            .insert("epochs_in_interval".to_owned(), gen.subschema_for::<u32>());
        object_validation
            .required
            .insert("epochs_in_interval".to_owned());

        // PrimitiveDateTime does not implement JsonSchema. However it has a custom
        // serialization to string, so we just specify the schema to be String.
        object_validation.properties.insert(
            "current_epoch_start".to_owned(),
            gen.subschema_for::<String>(),
        );
        object_validation
            .required
            .insert("current_epoch_start".to_owned());

        object_validation.properties.insert(
            "current_epoch_id".to_owned(),
            gen.subschema_for::<EpochId>(),
        );
        object_validation
            .required
            .insert("current_epoch_id".to_owned());

        object_validation
            .properties
            .insert("epoch_length".to_owned(), gen.subschema_for::<Duration>());
        object_validation.required.insert("epoch_length".to_owned());

        object_validation.properties.insert(
            "total_elapsed_epochs".to_owned(),
            gen.subschema_for::<EpochId>(),
        );
        object_validation
            .required
            .insert("total_elapsed_epochs".to_owned());

        Schema::Object(schema_object)
    }
}

impl Interval {
    /// Initialize epoch in the contract with default values.
    pub fn init_interval(epochs_in_interval: u32, epoch_length: Duration, env: &Env) -> Self {
        Interval {
            id: 0,
            epochs_in_interval,
            // I really don't see a way for this to fail, unless the blockchain is lying to us
            current_epoch_start: OffsetDateTime::from_unix_timestamp(
                env.block.time.seconds() as i64
            )
            .expect("Invalid timestamp from env.block.time"),
            current_epoch_id: 0,
            epoch_length,
            total_elapsed_epochs: 0,
        }
    }

    pub const fn current_epoch_id(&self) -> EpochId {
        self.current_epoch_id
    }

    pub const fn current_interval_id(&self) -> IntervalId {
        self.id
    }

    pub const fn epochs_in_interval(&self) -> u32 {
        self.epochs_in_interval
    }

    pub fn force_change_epochs_in_interval(&mut self, epochs_in_interval: u32) {
        self.epochs_in_interval = epochs_in_interval;
        if self.current_epoch_id >= epochs_in_interval {
            // we have to go to the next interval as we can't
            // have the same (interval, epoch) combo as we had in the past
            self.id += self.current_epoch_id / epochs_in_interval;
            self.current_epoch_id %= epochs_in_interval;
        }
    }

    pub fn change_epoch_length(&mut self, epoch_length: Duration) {
        self.epoch_length = epoch_length
    }

    pub const fn current_epoch_absolute_id(&self) -> u32 {
        // since we count epochs starting from 0, if n epochs have elapsed, the current one has absolute id of n
        self.total_elapsed_epochs
    }

    #[inline]
    pub fn is_current_epoch_over(&self, env: &Env) -> bool {
        self.current_epoch_end_unix_timestamp() <= env.block.time.seconds() as i64
    }

    pub fn secs_until_current_epoch_end(&self, env: &Env) -> i64 {
        if self.is_current_epoch_over(env) {
            0
        } else {
            self.current_epoch_end_unix_timestamp() - env.block.time.seconds() as i64
        }
    }

    #[inline]
    pub fn is_current_interval_over(&self, env: &Env) -> bool {
        self.current_interval_end_unix_timestamp() <= env.block.time.seconds() as i64
    }

    pub fn secs_until_current_interval_end(&self, env: &Env) -> i64 {
        if self.is_current_interval_over(env) {
            0
        } else {
            self.current_interval_end_unix_timestamp() - env.block.time.seconds() as i64
        }
    }

    pub fn current_epoch_in_progress(&self, env: &Env) -> bool {
        let block_time = env.block.time.seconds() as i64;
        self.current_epoch_start_unix_timestamp() <= block_time
            && block_time < self.current_epoch_end_unix_timestamp()
    }

    pub fn update_epoch_duration(&mut self, secs: u64) {
        self.epoch_length = Duration::from_secs(secs);
    }

    pub const fn epoch_length_secs(&self) -> u64 {
        self.epoch_length.as_secs()
    }

    /// Returns the next epoch. If if would result in advancing the interval,
    /// the relevant changes are applied.
    #[must_use]
    pub fn advance_epoch(&self) -> Self {
        // remember we start from 0th epoch, so if we're supposed to have 100 epochs in interval,
        // epoch 99 is going to be the last one
        if self.current_epoch_id == self.epochs_in_interval - 1 {
            Interval {
                id: self.id + 1,
                epochs_in_interval: self.epochs_in_interval,
                current_epoch_start: self.current_epoch_end(),
                current_epoch_id: 0,
                epoch_length: self.epoch_length,
                total_elapsed_epochs: self.total_elapsed_epochs + 1,
            }
        } else {
            Interval {
                id: self.id,
                epochs_in_interval: self.epochs_in_interval,
                current_epoch_start: self.current_epoch_end(),
                current_epoch_id: self.current_epoch_id + 1,
                epoch_length: self.epoch_length,
                total_elapsed_epochs: self.total_elapsed_epochs + 1,
            }
        }
    }

    /// Returns the starting datetime of this interval.
    pub const fn current_epoch_start(&self) -> OffsetDateTime {
        self.current_epoch_start
    }

    /// Returns the length of this interval.
    pub const fn epoch_length(&self) -> Duration {
        self.epoch_length
    }

    /// Returns the ending datetime of the current epoch.
    pub fn current_epoch_end(&self) -> OffsetDateTime {
        self.current_epoch_start + self.epoch_length
    }

    pub fn epochs_until_interval_end(&self) -> u32 {
        self.epochs_in_interval - self.current_epoch_id
    }

    /// Returns the ending datetime of the current interval.
    pub fn current_interval_end(&self) -> OffsetDateTime {
        self.current_epoch_start + self.epochs_until_interval_end() * self.epoch_length
    }

    /// Returns the unix timestamp of the start of the current epoch.
    pub const fn current_epoch_start_unix_timestamp(&self) -> i64 {
        self.current_epoch_start().unix_timestamp()
    }

    /// Returns the unix timestamp of the end of the current epoch.
    #[inline]
    pub fn current_epoch_end_unix_timestamp(&self) -> i64 {
        self.current_epoch_end().unix_timestamp()
    }

    /// Returns the unix timestamp of the end of the current interval.
    #[inline]
    pub fn current_interval_end_unix_timestamp(&self) -> i64 {
        self.current_interval_end().unix_timestamp()
    }
}

impl Display for Interval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let length = self.epoch_length_secs();
        let full_hours = length / 3600;
        let rem = length % 3600;
        write!(
            f,
            "Interval {}: epoch {}/{} (current epoch begun at: {}; epoch lengths: {}h {}s)",
            self.id,
            self.current_epoch_id + 1,
            self.epochs_in_interval,
            self.current_epoch_start,
            full_hours,
            rem
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CurrentIntervalResponse {
    pub interval: Interval,
    pub current_blocktime: u64,
    pub is_current_interval_over: bool,
    pub is_current_epoch_over: bool,
}

impl CurrentIntervalResponse {
    pub fn new(interval: Interval, env: Env) -> Self {
        CurrentIntervalResponse {
            interval,
            current_blocktime: env.block.time.seconds(),
            is_current_interval_over: interval.is_current_interval_over(&env),
            is_current_epoch_over: interval.is_current_epoch_over(&env),
        }
    }

    pub fn time_until_current_epoch_end(&self) -> Duration {
        if self.is_current_epoch_over {
            Duration::from_secs(0)
        } else {
            let remaining_secs =
                self.interval.current_epoch_end_unix_timestamp() - self.current_blocktime as i64;
            // this should never be negative, but better safe than sorry and guard ourselves against that case
            if remaining_secs <= 0 {
                Duration::from_secs(0)
            } else {
                Duration::from_secs(remaining_secs as u64)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct PendingEpochEventsResponse {
    pub seconds_until_executable: i64,
    pub events: Vec<PendingEpochEvent>,
    pub start_next_after: Option<u32>,
}

impl PendingEpochEventsResponse {
    pub fn new(
        seconds_until_executable: i64,
        events: Vec<PendingEpochEvent>,
        start_next_after: Option<u32>,
    ) -> Self {
        PendingEpochEventsResponse {
            seconds_until_executable,
            events,
            start_next_after,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct PendingIntervalEventsResponse {
    pub seconds_until_executable: i64,
    pub events: Vec<PendingIntervalEvent>,
    pub start_next_after: Option<u32>,
}

impl PendingIntervalEventsResponse {
    pub fn new(
        seconds_until_executable: i64,
        events: Vec<PendingIntervalEvent>,
        start_next_after: Option<u32>,
    ) -> Self {
        PendingIntervalEventsResponse {
            seconds_until_executable,
            events,
            start_next_after,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_env;
    use rand_chacha::rand_core::{RngCore, SeedableRng};

    #[test]
    fn advancing_epoch() {
        // just advancing epoch
        let interval = Interval {
            id: 0,
            epochs_in_interval: 100,
            current_epoch_start: time::macros::datetime!(2021-08-23 12:00 UTC),
            current_epoch_id: 23,
            epoch_length: Duration::from_secs(60 * 60),
            total_elapsed_epochs: 0,
        };
        let expected = Interval {
            id: 0,
            epochs_in_interval: 100,
            current_epoch_start: time::macros::datetime!(2021-08-23 13:00 UTC),
            current_epoch_id: 24,
            epoch_length: Duration::from_secs(60 * 60),
            total_elapsed_epochs: 1,
        };
        assert_eq!(expected, interval.advance_epoch());

        // results in advancing interval
        let interval = Interval {
            id: 0,
            epochs_in_interval: 100,
            current_epoch_start: time::macros::datetime!(2021-08-23 12:00 UTC),
            current_epoch_id: 99,
            epoch_length: Duration::from_secs(60 * 60),
            total_elapsed_epochs: 42,
        };
        let expected = Interval {
            id: 1,
            epochs_in_interval: 100,
            current_epoch_start: time::macros::datetime!(2021-08-23 13:00 UTC),
            current_epoch_id: 0,
            epoch_length: Duration::from_secs(60 * 60),
            total_elapsed_epochs: 43,
        };

        assert_eq!(expected, interval.advance_epoch())
    }

    #[test]
    fn checking_for_epoch_ends() {
        let env = mock_env();

        // epoch just begun
        let interval = Interval {
            id: 0,
            epochs_in_interval: 100,
            current_epoch_start: OffsetDateTime::from_unix_timestamp(
                env.block.time.seconds() as i64 - 100,
            )
            .unwrap(),
            current_epoch_id: 23,
            epoch_length: Duration::from_secs(60 * 60),
            total_elapsed_epochs: 0,
        };
        assert!(!interval.is_current_epoch_over(&env));

        // current time == current epoch start
        let mut interval = interval;
        interval.current_epoch_start =
            OffsetDateTime::from_unix_timestamp(env.block.time.seconds() as i64).unwrap();
        assert!(!interval.is_current_epoch_over(&env));

        // epoch HASN'T yet begun (weird edge case, but can happen if we decide to manually adjust things)
        let mut interval = interval;
        interval.current_epoch_start =
            OffsetDateTime::from_unix_timestamp(env.block.time.seconds() as i64 + 100).unwrap();
        assert!(!interval.is_current_epoch_over(&env));

        // current_time = EXACTLY end of the epoch
        let mut interval = interval;
        interval.current_epoch_start =
            OffsetDateTime::from_unix_timestamp(env.block.time.seconds() as i64).unwrap()
                - interval.epoch_length;
        assert!(interval.is_current_epoch_over(&env));

        // revert time a bit more
        interval.current_epoch_start -= Duration::from_secs(42);
        assert!(interval.is_current_epoch_over(&env));

        // revert by A LOT -> epoch still should be in finished state
        interval.current_epoch_start -= Duration::from_secs(5 * 31 * 60 * 60);
        assert!(interval.is_current_epoch_over(&env));
    }

    #[test]
    fn interval_end() {
        let mut interval = Interval {
            id: 0,
            epochs_in_interval: 100,
            current_epoch_start: time::macros::datetime!(2021-08-23 12:00 UTC),
            current_epoch_id: 99,
            epoch_length: Duration::from_secs(60 * 60),
            total_elapsed_epochs: 0,
        };

        assert_eq!(
            interval.current_epoch_start + interval.epoch_length,
            interval.current_interval_end()
        );

        interval.current_epoch_id -= 1;
        assert_eq!(
            interval.current_epoch_start + 2 * interval.epoch_length,
            interval.current_interval_end()
        );

        interval.current_epoch_id -= 10;
        assert_eq!(
            interval.current_epoch_start + 12 * interval.epoch_length,
            interval.current_interval_end()
        );

        interval.current_epoch_id = 0;
        assert_eq!(
            interval.current_epoch_start + interval.epochs_in_interval * interval.epoch_length,
            interval.current_interval_end()
        );
    }

    #[test]
    fn checking_for_interval_ends() {
        let env = mock_env();

        let epoch_length = Duration::from_secs(60 * 60);

        let mut interval = Interval {
            id: 0,
            epochs_in_interval: 100,
            current_epoch_start: OffsetDateTime::from_unix_timestamp(
                env.block.time.seconds() as i64
            )
            .unwrap(),
            current_epoch_id: 98,
            epoch_length,
            total_elapsed_epochs: 0,
        };

        // current epoch just started (we still have to finish 2 epochs)
        assert!(!interval.is_current_interval_over(&env));

        // still need to finish the 99th epoch
        interval.current_epoch_start -= epoch_length;
        assert!(!interval.is_current_interval_over(&env));

        // it JUST finished
        interval.current_epoch_start -= epoch_length;
        assert!(interval.is_current_interval_over(&env));

        // nobody updated the interval data, but the current one should still be in finished state
        interval.current_epoch_start -= 10 * epoch_length;
        assert!(interval.is_current_interval_over(&env));
    }

    #[test]
    fn getting_current_full_epoch_id() {
        let env = mock_env();
        let dummy_seed = [42u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let epoch_length = Duration::from_secs(60 * 60);

        let mut interval = Interval::init_interval(100, epoch_length, &env);

        // normal situation
        for i in 0u32..2000 {
            assert_eq!(interval.current_epoch_absolute_id(), i);
            interval = interval.advance_epoch();
        }

        let mut interval = Interval::init_interval(100, epoch_length, &env);

        for i in 0u32..2000 {
            // every few epochs decide to change epochs in interval
            if i % 7 == 0 {
                let new_epochs_in_interval = (rng.next_u32() % 200) + 42;
                interval.force_change_epochs_in_interval(new_epochs_in_interval)
            }

            // make sure full epoch id is always monotonically increasing
            assert_eq!(interval.current_epoch_absolute_id(), i);

            interval = interval.advance_epoch();
        }
    }
}
