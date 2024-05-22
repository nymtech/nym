// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::WireguardPaths;
use crate::config::Config;
use crate::error::NymNodeError;
use std::path::Path;

// currently there are no upgrades
async fn try_upgrade_config<P: AsRef<Path>>(path: P) -> Result<(), NymNodeError> {
    use crate::config::*;
    use crate::error::KeyIOFailure;
    use nym_crypto::asymmetric::encryption::KeyPair;
    use nym_pemstore::store_keypair;
    use rand::rngs::OsRng;

    #[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct OldWireguardPaths {
        // pub keys:
    }

    #[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct OldWireguard {
        /// Specifies whether the wireguard service is enabled on this node.
        pub enabled: bool,

        /// Socket address this node will use for binding its wireguard interface.
        /// default: `0.0.0.0:51822`
        pub bind_address: SocketAddr,

        /// Ip address of the private wireguard network.
        /// default: `10.1.0.0`
        pub private_network_ip: IpAddr,

        /// Port announced to external clients wishing to connect to the wireguard interface.
        /// Useful in the instances where the node is behind a proxy.
        pub announced_port: u16,

        /// The prefix denoting the maximum number of the clients that can be connected via Wireguard.
        /// The maximum value for IPv4 is 32 and for IPv6 is 128
        pub private_network_prefix: u8,

        /// Paths for wireguard keys, client registries, etc.
        pub storage_paths: OldWireguardPaths,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct OldConfig {
        // additional metadata holding on-disk location of this config file
        #[serde(skip)]
        pub(crate) save_path: Option<PathBuf>,

        /// Human-readable ID of this particular node.
        pub id: String,

        /// Current mode of this nym-node.
        /// Expect this field to be changed in the future to allow running the node in multiple modes (i.e. mixnode + gateway)
        pub mode: NodeMode,

        pub host: Host,

        pub mixnet: Mixnet,

        /// Storage paths to persistent nym-node data, such as its long term keys.
        pub storage_paths: NymNodePaths,

        #[serde(default)]
        pub http: Http,

        pub wireguard: OldWireguard,

        pub mixnode: MixnodeConfig,

        pub entry_gateway: EntryGatewayConfig,

        pub exit_gateway: ExitGatewayConfig,

        #[serde(default)]
        pub logging: LoggingSettings,
    }

    impl NymConfigTemplate for OldConfig {
        fn template(&self) -> &'static str {
            CONFIG_TEMPLATE
        }
    }

    impl OldConfig {
        fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
            let path = path.as_ref();
            let mut loaded: OldConfig = read_config_from_toml_file(path).map_err(|source| {
                NymNodeError::ConfigLoadFailure {
                    path: path.to_path_buf(),
                    source,
                }
            })?;
            loaded.save_path = Some(path.to_path_buf());
            debug!("loaded config file from {}", path.display());
            Ok(loaded)
        }
    }

    fn initialise(config: &Wireguard) -> std::io::Result<()> {
        let mut rng = OsRng;
        let x25519_keys = KeyPair::new(&mut rng);

        store_keypair(
            &x25519_keys,
            &config.storage_paths.x25519_wireguard_storage_paths(),
        )?;

        Ok(())
    }

    let old_cfg = OldConfig::read_from_path(&path)?;
    let wireguard = Wireguard {
        enabled: old_cfg.wireguard.enabled,
        bind_address: old_cfg.wireguard.bind_address,
        private_ip: old_cfg.wireguard.private_network_ip,
        announced_port: old_cfg.wireguard.announced_port,
        private_network_prefix: old_cfg.wireguard.private_network_prefix,
        storage_paths: WireguardPaths::new(Config::default_data_directory(path)?),
    };
    initialise(&wireguard).map_err(|err| KeyIOFailure::KeyPairStoreFailure {
        keys: "wg-x25519-dh".to_string(),
        paths: wireguard.storage_paths.x25519_wireguard_storage_paths(),
        err,
    })?;
    let cfg = Config {
        save_path: old_cfg.save_path,
        id: old_cfg.id,
        mode: old_cfg.mode,
        host: old_cfg.host,
        mixnet: old_cfg.mixnet,
        storage_paths: old_cfg.storage_paths,
        http: old_cfg.http,
        wireguard,
        mixnode: old_cfg.mixnode,
        entry_gateway: old_cfg.entry_gateway,
        exit_gateway: old_cfg.exit_gateway,
        logging: old_cfg.logging,
    };

    cfg.save()?;

    Ok(())
}

pub async fn try_load_current_config<P: AsRef<Path>>(
    config_path: P,
) -> Result<Config, NymNodeError> {
    if let Ok(cfg) = Config::read_from_toml_file(config_path.as_ref()) {
        return Ok(cfg);
    }

    try_upgrade_config(config_path.as_ref()).await?;
    Config::read_from_toml_file(config_path)
}
