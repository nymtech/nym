// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "base64")]
pub mod base64 {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let s = <String>::deserialize(deserializer)?;
        STANDARD.decode(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "bs58")]
pub mod bs58 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&::bs58::encode(bytes).into_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(deserializer)?;
        ::bs58::decode(&s)
            .into_vec()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "date")]
pub mod date {
    use serde::ser::Error;
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
    use time::format_description::{modifier, BorrowedFormatItem, Component};
    use time::Date;

    // simple YYYY-MM-DD
    pub const DATE_FORMAT: &[BorrowedFormatItem<'_>] = &[
        BorrowedFormatItem::Component(Component::Year(modifier::Year::default())),
        BorrowedFormatItem::Literal(b"-"),
        BorrowedFormatItem::Component(Component::Month(modifier::Month::default())),
        BorrowedFormatItem::Literal(b"-"),
        BorrowedFormatItem::Component(Component::Day(modifier::Day::default())),
    ];

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Date, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        Date::parse(&s, DATE_FORMAT).map_err(de::Error::custom)
    }

    pub fn serialize<S>(datetime: &Date, serializer: S) -> Result<S::Ok, S::Error>
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
