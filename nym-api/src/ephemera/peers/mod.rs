use std::collections::HashMap;
use std::str::FromStr;

use anyhow::anyhow;

use crate::ephemera::client::Client;
use crate::support::nyxd;
use ephemera::configuration::Configuration;
use ephemera::crypto::{EphemeraKeypair, EphemeraPublicKey, Keypair, PublicKey};

pub(crate) type PeerId = String;

pub mod members;

#[derive(Debug, Clone)]
pub struct NymPeer {
    pub cosmos_address: String,
    pub ip_address: String,
    pub public_key: PublicKey,
    pub peer_id: PeerId,
}

impl NymPeer {
    pub(crate) fn new(
        cosmos_address: String,
        ip_address: String,
        public_key: PublicKey,
        peer_id: PeerId,
    ) -> Self {
        Self {
            cosmos_address,
            ip_address,
            public_key,
            peer_id,
        }
    }
}

// Information about other Nym-Apis.
pub(crate) struct NymApiEphemeraPeerInfo {
    pub(crate) local_peer: NymPeer,
    pub(crate) peers: HashMap<PeerId, NymPeer>,
}

impl NymApiEphemeraPeerInfo {
    fn new(local_peer: NymPeer, peers: HashMap<PeerId, NymPeer>) -> Self {
        Self { local_peer, peers }
    }

    pub(crate) fn get_peers_count(&self) -> usize {
        self.peers.len()
    }

    pub(crate) async fn from_ephemera_dev_cluster_conf(
        conf: &Configuration,
        nyxd_client: nyxd::Client,
    ) -> anyhow::Result<NymApiEphemeraPeerInfo> {
        let node_info = conf.node.clone();

        let keypair = bs58::decode(&node_info.private_key).into_vec().unwrap();
        let keypair = Keypair::from_bytes(&keypair).unwrap();
        let local_peer_id = keypair.public_key().to_base58();

        let mut peers = HashMap::new();
        for peer_info in nyxd_client.get_ephemera_peers().await? {
            let public_key = PublicKey::from_str(&peer_info.public_key)?;

            let peer = NymPeer::new(
                peer_info.cosmos_address.to_string(),
                peer_info.ip_address,
                public_key,
                peer_info.public_key.clone(),
            );

            peers.insert(peer_info.public_key, peer);
        }

        let local_peer = peers
            .get(&local_peer_id)
            .ok_or(anyhow!("Failed to get local peer"))?
            .clone();
        let peer_info = NymApiEphemeraPeerInfo::new(local_peer, peers);
        Ok(peer_info)
    }
}
