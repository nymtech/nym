// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum::extract::ConnectInfo;
use axum::middleware::AddExtension;
use axum::serve::Serve;
use axum::Router;
use std::net::SocketAddr;

pub use router::{api, HttpServerConfig, NymNodeRouter};

pub mod error;
pub mod helpers;
pub mod router;
pub mod state;

type InnerService = IntoMakeServiceWithConnectInfo<Router, SocketAddr>;
type ConnectInfoExt = AddExtension<Router, ConnectInfo<SocketAddr>>;
pub type NymNodeHttpServer = Serve<InnerService, ConnectInfoExt>;
