use nym_crypto::asymmetric::identity::{PrivateKey, PublicKey, Signature};
use nym_mixnet_contract_common::NodeId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::LazyLock, time::SystemTime};

static NETWORK_MONITORS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let mut nm = HashSet::new();
    nm.insert("5VsPyLbsBCq9PAMWmjKkToteVAKNabNqex6QwDf5fWzt".to_string());
    nm
});

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct NodeResult {
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

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum MonitorResults {
    Mixnode(Vec<NodeResult>),
    Gateway(Vec<NodeResult>),
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MonitorMessage {
    results: Vec<NodeResult>,
    signature: String,
    signer: String,
    timestamp: i64,
}

impl MonitorMessage {
    fn message_to_sign(results: &[NodeResult], timestamp: i64) -> Vec<u8> {
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

    pub fn new(results: Vec<NodeResult>, private_key: &PrivateKey) -> Self {
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

    pub fn results(&self) -> &[NodeResult] {
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
