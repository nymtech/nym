// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::Router;
use axum::extract::ConnectInfo;
use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum::middleware::AddExtension;
use axum::serve::Serve;
use std::net::SocketAddr;

pub use router::{HttpServerConfig, NymNodeRouter, api};

pub mod error;
pub mod helpers;
pub mod router;
pub mod state;

type InnerService = IntoMakeServiceWithConnectInfo<Router, SocketAddr>;
type ConnectInfoExt = AddExtension<Router, ConnectInfo<SocketAddr>>;
pub type NymNodeHttpServer = Serve<InnerService, ConnectInfoExt>;
