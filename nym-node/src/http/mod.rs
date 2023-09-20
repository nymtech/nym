// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymNodeError;
use axum::routing::IntoMakeService;
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use hyper::server::conn::AddrIncoming;
use hyper::Server;
use std::net::SocketAddr;

mod api;
mod landing_page;

// TODO: can it be made nicer?
pub type NymNodeHTTPServer = Server<AddrIncoming, IntoMakeService<Router>>;

pub struct NymNodeRouter {
    inner: Router,
}

impl NymNodeRouter {
    pub fn create_server(
        self,
        bind_address: &SocketAddr,
    ) -> Result<NymNodeHTTPServer, NymNodeError> {
        Ok(axum::Server::try_bind(bind_address)
            .map_err(|source| NymNodeError::HttpBindFailure {
                bind_address: *bind_address,
                source,
            })?
            .serve(self.inner.into_make_service()))
    }
}
