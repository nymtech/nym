// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// use crate::error::PostgresScraperError;
// use nyxd_scraper_shared::Any;
// use sqlx::types::JsonValue;
//
// pub(crate) fn proto_to_json(proto: &Any) -> Result<JsonValue, PostgresScraperError> {
//     todo!()
// }

use nyxd_scraper_shared::Any;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct PlaceholderMessage {
    #[serde(rename = "@type")]
    pub(crate) typ: String,

    pub(crate) placeholder: String,
}

impl<'a> From<&'a Any> for PlaceholderMessage {
    fn from(value: &'a Any) -> Self {
        PlaceholderMessage {
            typ: value.type_url.to_ascii_lowercase(),
            placeholder: "PLACEHOLDER CONTENT - TODO: IMPLEMENT PROPER PROTO -> JSON PARSING"
                .to_string(),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct PlaceholderStruct {
    pub(crate) typ: String,
    pub(crate) placeholder: String,
}

impl PlaceholderStruct {
    pub(crate) fn new<T>(_: T) -> Self {
        PlaceholderStruct {
            typ: std::any::type_name::<T>().to_string(),
            placeholder: "PLACEHOLDER CONTENT - SOMETHING IS MISSING serde DERIVES".to_string(),
        }
    }
}
