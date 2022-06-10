// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::{response, Request, Response};
use std::io::Cursor;

use crate::storage::error::NetworkStatisticsStorageError;

pub type Result<T> = std::result::Result<T, NetworkStatisticsAPIError>;

#[derive(Debug, thiserror::Error)]
pub enum NetworkStatisticsAPIError {
    #[error("{0}")]
    RocketError(#[from] Box<rocket::Error>),

    #[error("{0}")]
    StorageError(#[from] NetworkStatisticsStorageError),
}

impl<'r, 'o: 'r> Responder<'r, 'o> for NetworkStatisticsAPIError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        let err_msg = self.to_string();
        Response::build()
            .header(ContentType::Plain)
            .sized_body(err_msg.len(), Cursor::new(err_msg))
            .status(Status::BadRequest)
            .ok()
    }
}
