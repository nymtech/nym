//! This is used to create local peers configuration file. Useful for local cluster development.

use crate::cli::PEERS_CONFIG_FILE;
use clap::Parser;

use crate::config::Configuration;
use crate::crypto::{EphemeraKeypair, EphemeraPublicKey, Keypair};
use crate::membership::PeerSetting;
use crate::network::members::ConfigPeers;

#[derive(Debug, Clone, Parser)]
pub struct CreateLocalPeersConfiguration;

impl CreateLocalPeersConfiguration {
    /// # Panics
    /// Panics if the configuration file cannot be written.
    pub fn execute(self) {
        let peers = Self::from_ephemera_dev_cluster_conf().unwrap();
        let config_peers = ConfigPeers::new(peers);

        let peers_conf_path = Configuration::ephemera_root_dir(None)
            .unwrap()
            .join(PEERS_CONFIG_FILE);

        config_peers.try_write(peers_conf_path).unwrap();
    }

    //LOCAL DEV CLUSTER ONLY
    //Get peers from dev Ephemera cluster config files
    pub(crate) fn from_ephemera_dev_cluster_conf() -> anyhow::Result<Vec<PeerSetting>> {
        let ephemera_root_dir = Configuration::ephemera_root_dir(None).unwrap();

        let mut peers = vec![];

        let home_dir = std::fs::read_dir(ephemera_root_dir)?;
        for entry in home_dir {
            let path = entry?.path();
            if path.is_dir() {
                let cosmos_address = path.file_name().unwrap().to_str().unwrap();

                println!("Reading peer info config from node {cosmos_address}",);

                let conf = Configuration::try_load_from_home_dir().unwrap_or_else(|_| {
                    panic!("Error loading configuration for node {cosmos_address}")
                });

                let node_info = conf.node;

                let keypair = bs58::decode(&node_info.private_key).into_vec().unwrap();
                let keypair = Keypair::from_bytes(&keypair).unwrap();

                let peer = PeerSetting {
                    cosmos_address: cosmos_address.to_string(),
                    address: format!("/ip4/{}/tcp/{}", node_info.ip, conf.libp2p.port),
                    public_key: keypair.public_key().to_base58(),
                };
                peers.push(peer);

                println!("Loaded config for node {cosmos_address}",);
            }
        }

        Ok(peers)
    }
}
