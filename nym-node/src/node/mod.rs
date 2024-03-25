// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::description::{load_node_description, save_node_description};
use crate::node::helpers::{
    load_ed25519_identity_keypair, load_key, load_x25519_sphinx_keypair,
    store_ed25519_identity_keypair, store_key, store_keypair, store_x25519_sphinx_keypair,
};
use crate::node::http::{sign_host_details, system_info::get_system_info};
use nym_bin_common::bin_info_owned;
use nym_crypto::asymmetric::{encryption, identity};
use nym_gateway::Gateway;
use nym_mixnode::MixNode;
use nym_network_requester::{
    set_active_gateway, setup_fs_gateways_storage, store_gateway_details, CustomGatewayDetails,
    GatewayDetails, GatewayRegistration,
};
use nym_node::config::entry_gateway::ephemeral_entry_gateway_config;
use nym_node::config::exit_gateway::ephemeral_exit_gateway_config;
use nym_node::config::mixnode::ephemeral_mixnode_config;
use nym_node::config::{Config, EntryGatewayConfig, ExitGatewayConfig, MixnodeConfig, NodeMode};
use nym_node::error::{EntryGatewayError, ExitGatewayError, MixnodeError, NymNodeError};
use nym_node_http_api::api::api_requests;
use nym_node_http_api::api::api_requests::v1::node::models::NodeDescription;
use nym_node_http_api::state::metrics::SharedMixingStats;
use nym_node_http_api::state::AppState;
use nym_node_http_api::{NymNodeHTTPServer, NymNodeRouter};
use nym_sphinx_acknowledgements::AckKey;
use nym_sphinx_addressing::Recipient;
use nym_task::{TaskClient, TaskManager};
use rand::rngs::OsRng;
use rand::{CryptoRng, RngCore};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, trace};
use zeroize::Zeroizing;

pub mod bonding_information;
pub mod description;
pub mod helpers;
pub(crate) mod http;

pub struct MixnodeData {
    mixing_stats: SharedMixingStats,
}

impl MixnodeData {
    pub fn initialise(_config: &MixnodeConfig) -> Result<(), MixnodeError> {
        Ok(())
    }

    fn new(_config: &MixnodeConfig) -> Result<MixnodeData, MixnodeError> {
        Ok(MixnodeData {
            mixing_stats: SharedMixingStats::new(),
        })
    }
}

pub struct EntryGatewayData {
    mnemonic: Zeroizing<bip39::Mnemonic>,
    client_storage: nym_gateway::node::PersistentStorage,
}

impl EntryGatewayData {
    pub fn initialise(
        config: &EntryGatewayConfig,
        custom_mnemonic: Option<Zeroizing<bip39::Mnemonic>>,
    ) -> Result<(), EntryGatewayError> {
        // SAFETY:
        // this unwrap is fine as 24 word count is a valid argument for generating entropy for a new bip39 mnemonic
        #[allow(clippy::unwrap_used)]
        let mnemonic = custom_mnemonic
            .unwrap_or_else(|| Zeroizing::new(bip39::Mnemonic::generate(24).unwrap()));
        config.storage_paths.save_mnemonic_to_file(&mnemonic)?;

        Ok(())
    }

    async fn new(config: &EntryGatewayConfig) -> Result<EntryGatewayData, EntryGatewayError> {
        Ok(EntryGatewayData {
            mnemonic: config.storage_paths.load_mnemonic_from_file()?,
            client_storage: nym_gateway::node::PersistentStorage::init(
                &config.storage_paths.clients_storage,
                config.debug.message_retrieval_limit,
            )
            .await
            .map_err(nym_gateway::GatewayError::from)?,
        })
    }
}

pub struct ExitGatewayData {
    // ideally we'd be storing all the keys here, but unfortunately due to how the service providers
    // are currently implemented, they will be loading the data themselves from the provided paths
}

impl ExitGatewayData {
    fn initialise_client_keys<R: RngCore + CryptoRng>(
        rng: &mut R,
        typ: &str,
        ed25519_paths: nym_pemstore::KeyPairPath,
        x25519_paths: nym_pemstore::KeyPairPath,
        ack_key_path: &Path,
    ) -> Result<(), ExitGatewayError> {
        let ed25519_keys = identity::KeyPair::new(rng);
        let x25519_keys = encryption::KeyPair::new(rng);
        let aes128ctr_key = AckKey::new(rng);

        store_keypair(
            &ed25519_keys,
            ed25519_paths,
            format!("{typ}-ed25519-identity"),
        )?;
        store_keypair(&x25519_keys, x25519_paths, format!("{typ}-x25519-dh"))?;
        store_key(&aes128ctr_key, ack_key_path, format!("{typ}-ack-key"))?;

        Ok(())
    }

    async fn initialise_client_gateway_storage(
        storage_path: &Path,
        registration: &GatewayRegistration,
    ) -> Result<(), ExitGatewayError> {
        // insert all required information into the gateways store
        // (I hate that we have to do it, but that's currently the simplest thing to do)
        let storage = setup_fs_gateways_storage(storage_path).await?;
        store_gateway_details(&storage, registration).await?;
        set_active_gateway(&storage, &registration.gateway_id().to_base58_string()).await?;
        Ok(())
    }

    pub async fn initialise_network_requester<R: RngCore + CryptoRng>(
        rng: &mut R,
        config: &ExitGatewayConfig,
        registration: &GatewayRegistration,
    ) -> Result<(), ExitGatewayError> {
        trace!("initialising network requester keys");
        Self::initialise_client_keys(
            rng,
            "network-requester",
            config
                .storage_paths
                .network_requester
                .ed25519_identity_storage_paths(),
            config
                .storage_paths
                .network_requester
                .x25519_diffie_hellman_storage_paths(),
            &config.storage_paths.network_requester.ack_key_file,
        )?;
        Self::initialise_client_gateway_storage(
            &config.storage_paths.network_requester.gateway_registrations,
            registration,
        )
        .await
    }

    pub async fn initialise_ip_packet_router_requester<R: RngCore + CryptoRng>(
        rng: &mut R,
        config: &ExitGatewayConfig,
        registration: &GatewayRegistration,
    ) -> Result<(), ExitGatewayError> {
        trace!("initialising ip packet router keys");
        Self::initialise_client_keys(
            rng,
            "ip-packet-router",
            config
                .storage_paths
                .ip_packet_router
                .ed25519_identity_storage_paths(),
            config
                .storage_paths
                .ip_packet_router
                .x25519_diffie_hellman_storage_paths(),
            &config.storage_paths.ip_packet_router.ack_key_file,
        )?;
        Self::initialise_client_gateway_storage(
            &config.storage_paths.ip_packet_router.gateway_registrations,
            registration,
        )
        .await
    }

    pub async fn initialise(
        config: &ExitGatewayConfig,
        public_key: identity::PublicKey,
    ) -> Result<(), ExitGatewayError> {
        // generate all the keys for NR and IPR
        let mut rng = OsRng;

        let gateway_details = GatewayDetails::Custom(CustomGatewayDetails::new(public_key)).into();

        // NR:
        Self::initialise_network_requester(&mut rng, config, &gateway_details).await?;

        // IPR:
        Self::initialise_ip_packet_router_requester(&mut rng, config, &gateway_details).await?;

        Ok(())
    }

    fn new(_config: &ExitGatewayConfig) -> Result<ExitGatewayData, ExitGatewayError> {
        Ok(ExitGatewayData {})
    }
}

pub(crate) struct NymNode {
    config: Config,
    description: NodeDescription,

    #[allow(dead_code)]
    mixnode: MixnodeData,

    entry_gateway: EntryGatewayData,

    #[allow(dead_code)]
    exit_gateway: ExitGatewayData,

    ed25519_identity_keys: Arc<identity::KeyPair>,
    x25519_sphinx_keys: Arc<encryption::KeyPair>,
}

impl NymNode {
    pub(crate) async fn initialise(
        config: &Config,
        custom_mnemonic: Option<Zeroizing<bip39::Mnemonic>>,
    ) -> Result<(), NymNodeError> {
        debug!("initialising nym-node with id: {}", config.id);
        let mut rng = OsRng;

        // global initialisation
        let ed25519_identity_keys = identity::KeyPair::new(&mut rng);
        let x25519_sphinx_keys = encryption::KeyPair::new(&mut rng);

        trace!("attempting to store ed25519 identity keypair");
        store_ed25519_identity_keypair(
            &ed25519_identity_keys,
            config.storage_paths.keys.ed25519_identity_storage_paths(),
        )?;

        trace!("attempting to store x25519 sphinx keypair");
        store_x25519_sphinx_keypair(
            &x25519_sphinx_keys,
            config.storage_paths.keys.x25519_sphinx_storage_paths(),
        )?;

        trace!("creating description file");
        save_node_description(
            &config.storage_paths.description,
            &NodeDescription::default(),
        )?;

        // mixnode initialisation
        MixnodeData::initialise(&config.mixnode)?;

        // entry gateway initialisation
        EntryGatewayData::initialise(&config.entry_gateway, custom_mnemonic)?;

        // exit gateway initialisation
        ExitGatewayData::initialise(&config.exit_gateway, *ed25519_identity_keys.public_key())
            .await?;

        config.save()
    }

    pub(crate) async fn new(config: Config) -> Result<Self, NymNodeError> {
        Ok(NymNode {
            ed25519_identity_keys: Arc::new(load_ed25519_identity_keypair(
                config.storage_paths.keys.ed25519_identity_storage_paths(),
            )?),
            x25519_sphinx_keys: Arc::new(load_x25519_sphinx_keypair(
                config.storage_paths.keys.x25519_sphinx_storage_paths(),
            )?),
            description: load_node_description(&config.storage_paths.description)?,
            mixnode: MixnodeData::new(&config.mixnode)?,
            entry_gateway: EntryGatewayData::new(&config.entry_gateway).await?,
            exit_gateway: ExitGatewayData::new(&config.exit_gateway)?,
            config,
        })
    }

    pub(crate) fn ed25519_identity_key(&self) -> &identity::PublicKey {
        self.ed25519_identity_keys.public_key()
    }

    pub(crate) fn x25519_sphinx_key(&self) -> &encryption::PublicKey {
        self.x25519_sphinx_keys.public_key()
    }

    fn start_mixnode(&self, task_client: TaskClient) -> Result<(), NymNodeError> {
        info!("going to start the nym-node in MIXNODE mode");

        let config = ephemeral_mixnode_config(self.config.clone())?;
        let mut mixnode = MixNode::new_loaded(
            config,
            Default::default(),
            self.ed25519_identity_keys.clone(),
            self.x25519_sphinx_keys.clone(),
        );
        mixnode.disable_http_server();
        mixnode.set_task_client(task_client);
        mixnode.set_mixing_stats(self.mixnode.mixing_stats.clone());

        tokio::spawn(async move { mixnode.run().await });
        Ok(())
    }

    async fn run_as_entry_gateway(self) -> Result<(), NymNodeError> {
        unimplemented!("ignore http");
        info!("going to start the nym-node in ENTRY GATEWAY mode");

        let config = ephemeral_entry_gateway_config(self.config, self.entry_gateway.mnemonic)?;
        let entry_gateway = Gateway::new_loaded(
            config,
            None,
            None,
            self.ed25519_identity_keys,
            self.x25519_sphinx_keys,
            self.entry_gateway.client_storage,
        );

        entry_gateway
            .run()
            .await
            .map_err(|source| NymNodeError::EntryGatewayFailure(source.into()))
    }

    async fn run_as_exit_gateway(self) -> Result<(), NymNodeError> {
        unimplemented!("ignore http");
        info!("going to start the nym-node in EXIT GATEWAY mode");

        let config = ephemeral_exit_gateway_config(self.config, self.entry_gateway.mnemonic)?;

        let exit_gateway = Gateway::new_loaded(
            config.gateway,
            Some(config.nr_opts),
            Some(config.ipr_opts),
            self.ed25519_identity_keys,
            self.x25519_sphinx_keys,
            self.entry_gateway.client_storage,
        );

        exit_gateway
            .run()
            .await
            .map_err(|source| NymNodeError::ExitGatewayFailure(source.into()))
    }

    pub(crate) fn build_http_server(&self) -> Result<NymNodeHTTPServer, NymNodeError> {
        let host_details = sign_host_details(
            &self.config,
            self.x25519_sphinx_keys.public_key(),
            &self.ed25519_identity_keys,
        )?;

        // mixnode info
        let mixnode_details = api_requests::v1::mixnode::models::Mixnode {};

        // entry gateway info
        let wireguard = if self.config.wireguard.enabled {
            Some(api_requests::v1::gateway::models::Wireguard {
                port: self.config.wireguard.announced_port,
                public_key: "placeholder key value".to_string(),
            })
        } else {
            None
        };
        let mixnet_websockets = Some(api_requests::v1::gateway::models::WebSockets {
            ws_port: self
                .config
                .entry_gateway
                .announce_ws_port
                .unwrap_or(self.config.entry_gateway.bind_address.port()),
            wss_port: self.config.entry_gateway.announce_wss_port,
        });
        let gateway_details = api_requests::v1::gateway::models::Gateway {
            client_interfaces: api_requests::v1::gateway::models::ClientInterfaces {
                wireguard,
                mixnet_websockets,
            },
        };

        // exit gateway info
        let nr_paths = &self.config.exit_gateway.storage_paths.network_requester;
        let nr_ed25519: identity::PublicKey = load_key(
            &nr_paths.public_ed25519_identity_key_file,
            "network requester ed25519",
        )?;

        let nr_x25519: encryption::PublicKey = load_key(
            &nr_paths.public_x25519_diffie_hellman_key_file,
            "network requester x25519",
        )?;

        let nr_details = api_requests::v1::network_requester::models::NetworkRequester {
            encoded_identity_key: nr_ed25519.to_base58_string(),
            encoded_x25519_key: nr_x25519.to_base58_string(),
            address: Recipient::new(
                nr_ed25519,
                nr_x25519,
                *self.ed25519_identity_keys.public_key(),
            )
            .to_string(),
        };

        let ipr_paths = &self.config.exit_gateway.storage_paths.ip_packet_router;
        let ipr_ed25519: identity::PublicKey = load_key(
            &ipr_paths.public_ed25519_identity_key_file,
            "ip packet router ed25519",
        )?;

        let ipr_x25519: encryption::PublicKey = load_key(
            &ipr_paths.public_x25519_diffie_hellman_key_file,
            "ip packet router x25519",
        )?;

        let ipr_details = api_requests::v1::ip_packet_router::models::IpPacketRouter {
            encoded_identity_key: ipr_ed25519.to_base58_string(),
            encoded_x25519_key: ipr_x25519.to_base58_string(),
            address: Recipient::new(
                ipr_ed25519,
                ipr_x25519,
                *self.ed25519_identity_keys.public_key(),
            )
            .to_string(),
        };
        let exit_policy_details =
            api_requests::v1::network_requester::exit_policy::models::UsedExitPolicy {
                enabled: true,
                upstream_source: self
                    .config
                    .exit_gateway
                    .upstream_exit_policy_url
                    .to_string(),
                last_updated: 0,
                // TODO: this will require some refactoring to actually retrieve the data from the embedded providers
                policy: None,
            };

        let mut config = nym_node_http_api::Config::new(bin_info_owned!(), host_details)
            .with_landing_page_assets(self.config.http.landing_page_assets_path.as_ref())
            .with_mixnode_details(mixnode_details)
            .with_gateway_details(gateway_details)
            .with_network_requester_details(nr_details)
            .with_ip_packet_router_details(ipr_details)
            .with_used_exit_policy(exit_policy_details)
            .with_description(self.description.clone());

        if self.config.http.expose_system_info {
            config = config.with_system_info(get_system_info(
                self.config.http.expose_system_hardware,
                self.config.http.expose_crypto_hardware,
            ))
        }
        match self.config.mode {
            NodeMode::Mixnode => config.api.v1_config.node.roles.mixnode_enabled = true,
            NodeMode::EntryGateway => config.api.v1_config.node.roles.gateway_enabled = true,
            NodeMode::ExitGateway => {
                config.api.v1_config.node.roles.network_requester_enabled = true;
                config.api.v1_config.node.roles.ip_packet_router_enabled = true;
            }
        }

        let app_state = AppState::new().with_mixing_stats(self.mixnode.mixing_stats.clone());

        Ok(NymNodeRouter::new(config, Some(app_state), None)
            .build_server(&self.config.http.bind_address)?)
    }

    pub(crate) async fn run(self) -> Result<(), NymNodeError> {
        let mut task_manager = TaskManager::default().named("NymNode");
        let http_server = self
            .build_http_server()?
            .with_task_client(task_manager.subscribe_named("http-server"));
        tokio::spawn(async move { http_server.run().await });

        match self.config.mode {
            NodeMode::Mixnode => {
                self.start_mixnode(task_manager.subscribe_named("mixnode"))?;
                let _ = task_manager.catch_interrupt().await;
                Ok(())
            }
            NodeMode::EntryGateway => self.run_as_entry_gateway().await,
            NodeMode::ExitGateway => self.run_as_exit_gateway().await,
        }
    }
}
