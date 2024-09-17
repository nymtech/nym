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
    use nym_serde_helpers::date::DATE_FORMAT;
    use serde::de::Visitor;
    use serde::ser::Error;
    use serde::{Deserializer, Serialize, Serializer};
    use std::fmt::Formatter;
    use time::format_description::well_known::Rfc3339;
    use time::format_description::{modifier, BorrowedFormatItem, Component};
    use time::OffsetDateTime;

    struct OffsetDateTimeVisitor;

    // copied from time library because they keep it private -.-
    const DEFAULT_OFFSET_DATE_TIME_FORMAT: &[BorrowedFormatItem<'_>] = &[
        BorrowedFormatItem::Compound(DATE_FORMAT),
        BorrowedFormatItem::Literal(b" "),
        BorrowedFormatItem::Compound(TIME_FORMAT),
        BorrowedFormatItem::Literal(b" "),
        BorrowedFormatItem::Compound(UTC_OFFSET_FORMAT),
    ];

    const TIME_FORMAT: &[BorrowedFormatItem<'_>] = &[
        BorrowedFormatItem::Component(Component::Hour(modifier::Hour::default())),
        BorrowedFormatItem::Literal(b":"),
        BorrowedFormatItem::Component(Component::Minute(modifier::Minute::default())),
        BorrowedFormatItem::Literal(b":"),
        BorrowedFormatItem::Component(Component::Second(modifier::Second::default())),
        BorrowedFormatItem::Literal(b"."),
        BorrowedFormatItem::Component(Component::Subsecond(modifier::Subsecond::default())),
    ];

    const UTC_OFFSET_FORMAT: &[BorrowedFormatItem<'_>] = &[
        BorrowedFormatItem::Component(Component::OffsetHour({
            let mut m = modifier::OffsetHour::default();
            m.sign_is_mandatory = true;
            m
        })),
        BorrowedFormatItem::Optional(&BorrowedFormatItem::Compound(&[
            BorrowedFormatItem::Literal(b":"),
            BorrowedFormatItem::Component(Component::OffsetMinute(
                modifier::OffsetMinute::default(),
            )),
            BorrowedFormatItem::Optional(&BorrowedFormatItem::Compound(&[
                BorrowedFormatItem::Literal(b":"),
                BorrowedFormatItem::Component(Component::OffsetSecond(
                    modifier::OffsetSecond::default(),
                )),
            ])),
        ])),
    ];

    impl<'de> Visitor<'de> for OffsetDateTimeVisitor {
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
}

// reimport the module to not break existing imports
pub(crate) use nym_serde_helpers::date as date_serde;
