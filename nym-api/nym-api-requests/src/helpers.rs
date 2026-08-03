// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use time::OffsetDateTime;

// just to have something, even if not accurate to generate the swagger docs
#[derive(JsonSchema)]
pub struct PlaceholderJsonSchemaImpl {}

pub(crate) const fn unix_epoch() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH
}

pub(crate) mod overengineered_offset_date_time_serde {
    use crate::helpers::unix_epoch;
    use serde::de::Visitor;
    use serde::ser::Error;
    use serde::{Deserializer, Serialize, Serializer};
    use std::fmt::Formatter;
    use time::format_description::well_known::Rfc3339;
    use time::format_description::BorrowedFormatItem;
    use time::macros::format_description;
    use time::OffsetDateTime;

    struct OffsetDateTimeVisitor;

    const DEFAULT_OFFSET_DATE_TIME_FORMAT: &[BorrowedFormatItem<'_>] = format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond] [offset_hour sign:mandatory][optional [:[offset_minute][optional [:[offset_second]]]]]"
    );

    impl Visitor<'_> for OffsetDateTimeVisitor {
        type Value = OffsetDateTime;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("an rfc3339 or human-readable `OffsetDateTime`")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // first try rfc3339, if that fails use default human-readable impl from time,
            // finally fallback to default unix epoch
            Ok(OffsetDateTime::parse(v, &Rfc3339).unwrap_or_else(|_| {
                OffsetDateTime::parse(v, &DEFAULT_OFFSET_DATE_TIME_FORMAT)
                    .unwrap_or_else(|_| unix_epoch())
            }))
        }
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(OffsetDateTimeVisitor)
    }

    pub(crate) fn serialize<S>(datetime: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // serialize it with human-readable format for compatibility with eclipse and nutella clients
        // in the future change it back to rfc3339
        datetime
            .format(&DEFAULT_OFFSET_DATE_TIME_FORMAT)
            .map_err(S::Error::custom)?
            .serialize(serializer)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde::Deserialize;
        use time::macros::datetime;

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Wrapper(
            #[serde(with = "crate::helpers::overengineered_offset_date_time_serde")] OffsetDateTime,
        );

        fn ser(dt: OffsetDateTime) -> String {
            serde_json::to_string(&Wrapper(dt)).unwrap()
        }

        fn de(s: &str) -> OffsetDateTime {
            let quoted = format!("\"{s}\"");
            serde_json::from_str::<Wrapper>(&quoted).unwrap().0
        }

        #[test]
        fn serializes_using_human_readable_format() {
            let dt = datetime!(2024-03-15 13:37:42.123456 +02:00);
            assert_eq!(ser(dt), "\"2024-03-15 13:37:42.123456 +02:00:00\"");
        }

        #[test]
        fn serializes_utc_offset() {
            let dt = datetime!(2024-01-01 00:00:00.0 +00:00);
            assert_eq!(ser(dt), "\"2024-01-01 00:00:00.0 +00:00:00\"");
        }

        #[test]
        fn deserializes_rfc3339() {
            let dt = de("2024-03-15T13:37:42.123456Z");
            assert_eq!(dt, datetime!(2024-03-15 13:37:42.123456 +00:00));
        }

        #[test]
        fn deserializes_rfc3339_with_offset() {
            let dt = de("2024-03-15T13:37:42.123456+02:00");
            assert_eq!(dt, datetime!(2024-03-15 13:37:42.123456 +02:00));
        }

        #[test]
        fn deserializes_default_human_readable_format() {
            let dt = de("2024-03-15 13:37:42.123456 +02:00");
            assert_eq!(dt, datetime!(2024-03-15 13:37:42.123456 +02:00));
        }

        #[test]
        fn falls_back_to_unix_epoch_on_garbage_input() {
            let dt = de("not a valid datetime");
            assert_eq!(dt, unix_epoch());
        }

        #[test]
        fn round_trips_through_serialize_then_deserialize() {
            let original = datetime!(2024-03-15 13:37:42.123456 +02:00);
            let serialized = ser(original);
            let stripped = serialized.trim_matches('"');
            assert_eq!(de(stripped), original);
        }
    }
}

// reimport the module to not break existing imports
pub(crate) use nym_serde_helpers::date as date_serde;
