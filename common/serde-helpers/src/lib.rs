// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// note: "use 'serde_with' alongside '#[serde_as(as = 'Base64')]' instead"
#[cfg(feature = "base64")]
pub mod base64 {
    use serde::{Deserializer, Serializer};
    use serde_with::{DeserializeAs, SerializeAs, base64::Base64};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        Base64: SerializeAs<T>,
        S: Serializer,
    {
        Base64::serialize_as(value, serializer)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        Base64: DeserializeAs<'de, T>,
        D: Deserializer<'de>,
    {
        Base64::deserialize_as(deserializer)
    }
}

// note: "use 'serde_with' alongside '#[serde_as(as = 'Base58')]' instead"
#[cfg(feature = "bs58")]
pub mod bs58 {
    use serde::{Deserializer, Serializer};
    use serde_with::{DeserializeAs, SerializeAs, base58::Base58};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        Base58: SerializeAs<T>,
        S: Serializer,
    {
        Base58::serialize_as(value, serializer)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        Base58: DeserializeAs<'de, T>,
        D: Deserializer<'de>,
    {
        Base58::deserialize_as(deserializer)
    }
}

// note: "use 'serde_with' alongside '#[serde_as(as = 'Hex')]' instead"
#[cfg(feature = "hex")]
pub mod hex {
    use serde::{Deserializer, Serializer};
    use serde_with::{DeserializeAs, SerializeAs, hex::Hex};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        Hex: SerializeAs<T>,
        S: Serializer,
    {
        Hex::serialize_as(value, serializer)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        Hex: DeserializeAs<'de, T>,
        D: Deserializer<'de>,
    {
        Hex::deserialize_as(deserializer)
    }
}

#[cfg(feature = "date")]
pub mod date {
    use serde::ser::Error;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
    use time::Date;
    use time::format_description::BorrowedFormatItem;
    use time::macros::format_description;

    // simple YYYY-MM-DD
    pub const DATE_FORMAT: &[BorrowedFormatItem<'_>] = format_description!("[year]-[month]-[day]");

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

#[cfg(test)]
mod tests {

    #[cfg(feature = "date")]
    #[cfg(test)]
    mod date_tests {
        use serde::{Deserialize, Serialize};
        use time::Date;
        use time::macros::date;

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Foo {
            #[serde(with = "crate::date")]
            date: Date,
        }

        #[test]
        fn date_serialisation() {
            let date = date!(2023 - 02 - 01);
            let foo = Foo { date };
            let ser = serde_json::to_string(&foo).unwrap();
            assert_eq!(ser, r#"{"date":"2023-02-01"}"#);

            let de: Foo = serde_json::from_str(&ser).unwrap();
            assert_eq!(de, foo);
        }
    }
}
