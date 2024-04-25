// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Gateway {
    #[serde(default)]
    pub enforces_zk_nyms: bool,

    pub client_interfaces: ClientInterfaces,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Wireguard {
    #[cfg_attr(feature = "openapi", schema(example = 51820, default = 51820))]
    pub port: u16,

    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ClientInterfaces {
    pub wireguard: Option<Wireguard>,

    pub mixnet_websockets: Option<WebSockets>,
    // pub mixnet_tcp:
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WebSockets {
    #[cfg_attr(feature = "openapi", schema(example = 9000, default = 9000))]
    pub ws_port: u16,

    pub wss_port: Option<u16>,
}
