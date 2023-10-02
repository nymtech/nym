// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct Gateway {
    pub client_interfaces: ClientInterfaces,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct Wireguard {
    #[schema(example = 1234, default = 51820)]
    pub port: u16,

    pub public_key: String,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct ClientInterfaces {
    pub wireguard: Option<Wireguard>,

    pub mixnet_websockets: Option<WebSockets>,
    // pub mixnet_tcp:
}

#[derive(Serialize, Debug, Clone, Copy, ToSchema)]
pub struct WebSockets {
    #[schema(example = 1234, default = 9000)]
    pub ws_port: u16,

    pub wss_port: Option<u16>,
}
