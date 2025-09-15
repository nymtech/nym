// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::unix_epoch;
use cosmwasm_std::Uint128;
use schemars::schema::{InstanceType, Schema, SchemaObject};
use schemars::{JsonSchema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};
use time::OffsetDateTime;
use utoipa::ToSchema;

#[derive(ToSchema)]
#[schema(title = "Coin")]
pub struct CoinSchema {
    pub denom: String,
    #[schema(value_type = String)]
    pub amount: Uint128,
}

pub fn de_rfc3339_or_default<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(time::serde::rfc3339::deserialize(deserializer).unwrap_or_else(|_| unix_epoch()))
}

// for all intents and purposes it's just OffsetDateTime, but we need JsonSchema...
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct OffsetDateTimeJsonSchemaWrapper(
    #[serde(
        default = "unix_epoch",
        with = "crate::helpers::overengineered_offset_date_time_serde"
    )]
    #[schema(inline)]
    pub OffsetDateTime,
);

impl Default for OffsetDateTimeJsonSchemaWrapper {
    fn default() -> Self {
        OffsetDateTimeJsonSchemaWrapper(unix_epoch())
    }
}

impl Display for OffsetDateTimeJsonSchemaWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl From<OffsetDateTimeJsonSchemaWrapper> for OffsetDateTime {
    fn from(value: OffsetDateTimeJsonSchemaWrapper) -> Self {
        value.0
    }
}

impl From<OffsetDateTime> for OffsetDateTimeJsonSchemaWrapper {
    fn from(value: OffsetDateTime) -> Self {
        OffsetDateTimeJsonSchemaWrapper(value)
    }
}

impl Deref for OffsetDateTimeJsonSchemaWrapper {
    type Target = OffsetDateTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OffsetDateTimeJsonSchemaWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// implementation taken from: https://github.com/GREsau/schemars/pull/207
impl JsonSchema for OffsetDateTimeJsonSchemaWrapper {
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        "DateTime".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            format: Some("date-time".into()),
            ..Default::default()
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn offset_date_time_json_schema_wrapper_serde_backwards_compat() {
        let mut dummy = OffsetDateTimeJsonSchemaWrapper::default();
        dummy.0 += Duration::from_millis(1);
        let ser = serde_json::to_string(&dummy).unwrap();

        assert_eq!("\"1970-01-01 00:00:00.001 +00:00:00\"", ser);

        let human_readable = "\"2024-05-23 07:41:02.756283766 +00:00:00\"";
        let rfc3339 = "\"2002-10-02T15:00:00Z\"";
        let rfc3339_offset = "\"2002-10-02T10:00:00-05:00\"";

        let de = serde_json::from_str::<OffsetDateTimeJsonSchemaWrapper>(human_readable).unwrap();
        assert_eq!(de.0.unix_timestamp(), 1716450062);

        let de = serde_json::from_str::<OffsetDateTimeJsonSchemaWrapper>(rfc3339).unwrap();
        assert_eq!(de.0.unix_timestamp(), 1033570800);

        let de = serde_json::from_str::<OffsetDateTimeJsonSchemaWrapper>(rfc3339_offset).unwrap();
        assert_eq!(de.0.unix_timestamp(), 1033570800);

        let de = serde_json::from_str::<OffsetDateTimeJsonSchemaWrapper>("\"nonsense\"").unwrap();
        assert_eq!(de.0.unix_timestamp(), 0);
    }
}
