// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::Router;
use axum::extract::ConnectInfo;
use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum::middleware::AddExtension;
use axum::serve::WithGracefulShutdown;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio_util::sync::WaitForCancellationFutureOwned;

pub use router::{HttpServerConfig, NymNodeRouter, api};

pub mod error;
pub mod helpers;
pub mod router;
pub mod state;

type MakeService = IntoMakeServiceWithConnectInfo<Router, SocketAddr>;
type InnerService = AddExtension<Router, ConnectInfo<SocketAddr>>;
pub type NymNodeHttpServer =
    WithGracefulShutdown<TcpListener, MakeService, InnerService, WaitForCancellationFutureOwned>;
