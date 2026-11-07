// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::ed25519;
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
    #[deprecated(note = "use specific port instead (tunnel or metadata service)")]
    #[cfg_attr(feature = "openapi", schema(example = 51822, default = 51822))]
    pub port: u16,

    #[cfg_attr(feature = "openapi", schema(example = 51822, default = 51822))]
    pub tunnel_port: u16,

    #[cfg_attr(feature = "openapi", schema(example = 51830, default = 51830))]
    pub metadata_port: u16,

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

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Bridges {
    pub client_params_path: String,
}

/// Domain separation from everything else the node identity key signs.
const BRIDGE_CLIENT_PARAMS_DOMAIN: &[u8] = b"nym-bridge-client-params-v1\n";

/// `client_bridge_params.json` exactly as read from disk, with a signature over its bytes.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SignedBridgeClientParams {
    pub payload: String,

    /// Base58 ed25519 signature over the domain prefix followed by the payload bytes.
    pub signature: String,
}

impl SignedBridgeClientParams {
    fn message(payload: &str) -> Vec<u8> {
        [BRIDGE_CLIENT_PARAMS_DOMAIN, payload.as_bytes()].concat()
    }

    pub fn new(payload: String, key: &ed25519::PrivateKey) -> Self {
        let signature = key.sign(Self::message(&payload)).to_base58_string();
        SignedBridgeClientParams { payload, signature }
    }

    pub fn verify(&self, key: &ed25519::PublicKey) -> bool {
        ed25519::Signature::from_base58_string(&self.signature)
            .is_ok_and(|sig| key.verify(Self::message(&self.payload), &sig).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_bridge_client_params_verify() {
        let mut rng = nym_test_utils::helpers::deterministic_rng();
        let key = ed25519::KeyPair::new(&mut rng);
        let signed =
            SignedBridgeClientParams::new("{\"transports\": []}\n".to_string(), key.private_key());
        assert!(signed.verify(key.public_key()));

        let mut flipped = signed.clone();
        flipped.payload = flipped.payload.replace("[]", "[ ]");
        assert!(!flipped.verify(key.public_key()));

        let other = ed25519::KeyPair::new(&mut rng);
        assert!(!signed.verify(other.public_key()));

        let mut malformed = signed;
        malformed.signature = "not base58 0OIl".to_string();
        assert!(!malformed.verify(key.public_key()));
    }
}
