// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "base64")]
pub mod base64 {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
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

#[cfg(feature = "hex")]
pub mod hex {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&::hex::encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(deserializer)?;
        ::hex::decode(&s).map_err(serde::de::Error::custom)
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
