// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::helpers::{
    load_ed25519_identity_keypair, load_x25519_sphinx_keypair, store_ed25519_identity_keypair,
    store_key, store_keypair, store_x25519_sphinx_keypair,
};
use nym_crypto::asymmetric::{encryption, identity};
use nym_gateway::Gateway;
use nym_mixnode::node::node_description::NodeDescription;
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
use nym_sphinx_acknowledgements::AckKey;
use rand::rngs::OsRng;
use rand::{CryptoRng, RngCore};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, trace};
use zeroize::Zeroizing;

pub mod bonding_information;
pub mod helpers;

struct MixnodeData {
    descriptor: NodeDescription,
}

impl MixnodeData {
    fn initialise(config: &MixnodeConfig) -> Result<(), MixnodeError> {
        NodeDescription::default()
            .save_to_file(&config.storage_paths.node_description)
            .map_err(|source| MixnodeError::DescriptionSaveFailure {
                path: config.storage_paths.node_description.clone(),
                source,
            })
    }

    fn new(config: &MixnodeConfig) -> Result<MixnodeData, MixnodeError> {
        Ok(MixnodeData {
            descriptor: NodeDescription::load_from_file(&config.storage_paths.node_description)
                .map_err(|source| MixnodeError::DescriptionLoadFailure {
                    path: config.storage_paths.node_description.clone(),
                    source,
                })?,
        })
    }
}

struct EntryGatewayData {
    mnemonic: Zeroizing<bip39::Mnemonic>,
    client_storage: nym_gateway::node::PersistentStorage,
}

impl EntryGatewayData {
    fn initialise(
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

struct ExitGatewayData {
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

    async fn initialise_network_requester<R: RngCore + CryptoRng>(
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

    async fn initialise_ip_packet_router_requester<R: RngCore + CryptoRng>(
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

    async fn initialise(
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

    async fn run_as_mixnode(self) -> Result<(), NymNodeError> {
        info!("going to start the nym-node in MIXNODE mode");

        let config = ephemeral_mixnode_config(self.config)?;
        let mut mixnode = MixNode::new_loaded(
            config,
            self.mixnode.descriptor,
            self.ed25519_identity_keys,
            self.x25519_sphinx_keys,
        );
        mixnode.run().await?;
        Ok(())
    }

    async fn run_as_entry_gateway(self) -> Result<(), NymNodeError> {
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

    pub(crate) async fn run(self) -> Result<(), NymNodeError> {
        match self.config.mode {
            NodeMode::Mixnode => self.run_as_mixnode().await,
            NodeMode::EntryGateway => self.run_as_entry_gateway().await,
            NodeMode::ExitGateway => self.run_as_exit_gateway().await,
        }
    }
}
