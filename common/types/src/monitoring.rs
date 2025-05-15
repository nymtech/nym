use nym_crypto::asymmetric::ed25519::{PrivateKey, PublicKey, Signature};
use nym_mixnet_contract_common::NodeId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::LazyLock, time::SystemTime};
use utoipa::ToSchema;

static NETWORK_MONITORS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let mut nm = HashSet::new();
    nm.insert("5VsPyLbsBCq9PAMWmjKkToteVAKNabNqex6QwDf5fWzt".to_string());
    nm
});

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, ToSchema)]
pub struct RouteResult {
    pub layer1: u32,
    pub layer2: u32,
    pub layer3: u32,
    pub gw: u32,
    pub success: bool,
}

impl RouteResult {
    pub fn new(layer1: u32, layer2: u32, layer3: u32, gw: u32, success: bool) -> Self {
        RouteResult {
            layer1,
            layer2,
            layer3,
            gw,
            success,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, ToSchema)]
pub struct NodeResult {
    #[schema(value_type = u32)]
    pub node_id: NodeId,
    pub identity: String,
    pub reliability: u8,
}

impl NodeResult {
    pub fn new(node_id: NodeId, identity: String, reliability: u8) -> Self {
        NodeResult {
            node_id,
            identity,
            reliability,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(untagged)]
pub enum MonitorResults {
    Node(Vec<NodeResult>),
    Route(Vec<RouteResult>),
}

#[derive(Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct MonitorMessage {
    results: MonitorResults,
    signature: String,
    signer: String,
    timestamp: i64,
}

impl MonitorMessage {
    fn message_to_sign(results: &MonitorResults, timestamp: i64) -> Vec<u8> {
        let mut msg = serde_json::to_vec(results).unwrap_or_default();
        msg.extend_from_slice(&timestamp.to_le_bytes());
        msg
    }

    pub fn timely(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;

        now - self.timestamp < 5
    }

    pub fn new(results: MonitorResults, private_key: &PrivateKey) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;

        let msg = Self::message_to_sign(&results, timestamp);
        let signature = private_key.sign(&msg);
        let public_key = private_key.public_key();

        MonitorMessage {
            results,
            signature: signature.to_base58_string(),
            signer: public_key.to_base58_string(),
            timestamp,
        }
    }

    pub fn is_in_allowed(&self) -> bool {
        NETWORK_MONITORS.contains(&self.signer)
    }

    pub fn results(&self) -> &MonitorResults {
        &self.results
    }

    pub fn verify(&self) -> bool {
        let msg = Self::message_to_sign(&self.results, self.timestamp);

        let signature = match Signature::from_base58_string(&self.signature) {
            Ok(sig) => sig,
            Err(_) => return false,
        };

        PublicKey::from_base58_string(&self.signer)
            .map(|pk| pk.verify(msg, &signature).is_ok())
            .unwrap_or(false)
    }
}
