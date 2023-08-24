// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::{response, Request, Response};
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::Responses;
use rocket_okapi::response::OpenApiResponderInner;
use rocket_okapi::util::ensure_status_code_exists;
use std::io::Cursor;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Database experienced an internal error - {0}")]
    InternalDatabaseError(#[from] sqlx::Error),

    #[error("SQL migrate error - {0}")]
    DatabaseMigrateError(#[from] sqlx::migrate::MigrateError),

    #[error("NyxdError - {0}")]
    NyxdError(#[from] nym_validator_client::nyxd::error::NyxdError),

    #[error("Invalid payment requested")]
    InvalidPaymentRequest,

    #[error("Bad deposit address")]
    BadAddress,

    #[error("Empty list of validators")]
    EmptyValidatorList,
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        let err_msg = self.to_string();
        Response::build()
            .header(ContentType::Plain)
            .sized_body(err_msg.len(), Cursor::new(err_msg))
            .status(Status::BadRequest)
            .ok()
    }
}

impl OpenApiResponderInner for Error {
    fn responses(_gen: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        let mut responses = Responses::default();
        ensure_status_code_exists(&mut responses, 404);
        Ok(responses)
    }
}
