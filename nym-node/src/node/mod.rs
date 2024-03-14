// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

mod helpers;

use crate::node::helpers::{store_ed25519_identity_keypair, store_x25519_sphinx_keypair};
use nym_crypto::asymmetric::{encryption, identity};
use nym_node::config::{Config, NodeMode};
use nym_node::error::NymNodeError;
use std::sync::Arc;
use tracing::{debug, info, trace};

pub(crate) struct NymNode {
    config: Config,

    ed25519_identity_keys: Arc<identity::KeyPair>,
    x25519_sphinx_keys: Arc<encryption::KeyPair>,
}

impl NymNode {
    pub(crate) fn initialise(config: &Config) -> Result<(), NymNodeError> {
        debug!("initialising nym-node with id: {}", config.id);
        let mut rng = rand::rngs::OsRng;

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

        config.save()
    }

    pub(crate) fn new(config: Config) -> Result<Self, NymNodeError> {
        todo!()
    }

    async fn run_as_mixnode(self) -> Result<(), NymNodeError> {
        info!("going to start the nym-node in MIXNODE mode");
        Ok(())
    }

    async fn run_as_entry_gateway(self) -> Result<(), NymNodeError> {
        info!("going to start the nym-node in ENTRY GATEWAY mode");
        Ok(())
    }

    async fn run_as_exit_gateway(self, ipr: bool) -> Result<(), NymNodeError> {
        info!("going to start the nym-node in EXIT GATEWAY mode (ipr: {ipr})");
        Ok(())
    }

    pub(crate) async fn run(self) -> Result<(), NymNodeError> {
        match self.config.mode {
            NodeMode::Mixnode => self.run_as_mixnode().await,
            NodeMode::EntryGateway => self.run_as_entry_gateway().await,
            NodeMode::ExitGatewayNR => self.run_as_exit_gateway(false).await,
            NodeMode::ExitGatewayIPR => self.run_as_exit_gateway(true).await,
        }
    }
}
