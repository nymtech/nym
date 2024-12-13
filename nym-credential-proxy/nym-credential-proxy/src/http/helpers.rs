// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::VpnApiError;
use crate::http::types::RequestError;
use axum::http::StatusCode;
use rand::rngs::OsRng;
use rand::RngCore;
use tracing::warn;
use uuid::Uuid;

pub fn random_uuid() -> Uuid {
    let mut bytes = [0u8; 16];
    let mut rng = OsRng;
    rng.fill_bytes(&mut bytes);
    Uuid::from_bytes(bytes)
}

pub fn db_failure<T>(err: VpnApiError, uuid: Uuid) -> Result<T, RequestError> {
    warn!("db failure: {err}");
    Err(RequestError::new_with_uuid(
        format!("oh no, something went wrong {err}"),
        uuid,
        StatusCode::INTERNAL_SERVER_ERROR,
    ))
}
