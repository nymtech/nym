// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum::Router;
use nym_http_api_common::middleware::logger;
use std::net::SocketAddr;

pub mod v1;

pub struct NymApiRouter {
    inner: Router,
}

impl NymApiRouter {
    pub fn new() -> NymApiRouter {
        // TODO: perhaps metrics:
        // https://github.com/tokio-rs/axum/blob/main/examples/prometheus-metrics/src/main.rs
        NymApiRouter {
            inner: Router::new().layer(axum::middleware::from_fn(logger)),
        }
    }

    pub fn into_make_service_with_connect_info(
        self,
    ) -> IntoMakeServiceWithConnectInfo<Router, SocketAddr> {
        self.inner.into_make_service_with_connect_info()
    }
}
