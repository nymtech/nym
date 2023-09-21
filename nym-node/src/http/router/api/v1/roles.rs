// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::router::api::Output;
use axum::extract::Query;
use axum::response::IntoResponse;

pub enum NodeRole {
    Mixnode {
        //
    },
    Gateway {
        //
    },
    NetworkRequester {
        //
    },
}

pub async fn roles(output: Query<Option<Output>>) -> impl IntoResponse {
    let output = output.unwrap_or_default();
    todo!()
}
