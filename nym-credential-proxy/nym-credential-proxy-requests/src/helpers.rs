// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use time::format_description::{modifier, BorrowedFormatItem, Component};

const DATE_FORMAT: &[BorrowedFormatItem<'_>] = &[
    BorrowedFormatItem::Component(Component::Year(modifier::Year::default())),
    BorrowedFormatItem::Literal(b"-"),
    BorrowedFormatItem::Component(Component::Month(modifier::Month::default())),
    BorrowedFormatItem::Literal(b"-"),
    BorrowedFormatItem::Component(Component::Day(modifier::Day::default())),
];

pub(crate) mod date_serde {
    use crate::helpers::DATE_FORMAT;
    use serde::ser::Error;
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
    use time::Date;

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Date, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        Date::parse(&s, DATE_FORMAT).map_err(de::Error::custom)
    }

    pub(crate) fn serialize<S>(datetime: &Date, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // serialize it with human-readable format for compatibility with eclipse and nutella clients
        // in the future change it back to rfc3339
        datetime
            .format(&DATE_FORMAT)
            .map_err(S::Error::custom)?
            .serialize(serializer)
    }
}
