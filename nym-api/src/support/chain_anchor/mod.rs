// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Trust in the Nyx chain, shared by every contract nym-api attests: one anchor (a proven read
//! through the trusted rpc node, or a light client seeded from a checkpoint) establishes the app
//! hash at a height, and each contract proves its own digest against it.

use crate::support::chain_anchor::checkpoint::get_trusted_checkpoint;
use crate::support::config::DirectoryConfigDebug;
use anyhow::Context;
use async_trait::async_trait;
use nym_contract_anchor::anchor::checkpoint::store::FileCheckpointStore;
use nym_contract_anchor::anchor::helpers::get_trusted_digest;
use nym_contract_anchor::anchor::proven::ProvenTrustAnchor;
use nym_contract_anchor::anchor::{
    nyx_default_options, LightClientAnchor, TrustAnchor, TrustedDigest,
};
use nym_contract_anchor::error::AnchorError;
use nym_directory_client::key::digest_item_key;
use nym_validator_client::nyxd::contract_traits::NymContractsProvider;
use nym_validator_client::nyxd::{AccountId, TendermintRpcClient};
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::path::Path;
use std::sync::Arc;
use tendermint::block::Height;
use tendermint::{chain, AppHash};

mod checkpoint;

/// The chain-level anchor every attested contract's [`ContractAnchor`] takes its app hashes from.
pub(crate) struct ChainAnchor {
    // the underlying anchors are built for a contract, but only their app hashes are used:
    // each contract proves its own digest in `ContractAnchor`
    anchor: Arc<dyn TrustAnchor + Send + Sync>,

    chain_id: chain::Id,

    query_client: QueryHttpRpcNyxdClient,
}

impl ChainAnchor {
    /// A proven anchor through the rpc node if `[directory.debug].trusted_rpc_node`, else a light
    /// client that persists its verified head to `on_disk_checkpoint_file`.
    pub(crate) async fn new(
        config: &DirectoryConfigDebug,
        query_client: QueryHttpRpcNyxdClient,
        on_disk_checkpoint_file: &Path,
    ) -> anyhow::Result<Self> {
        if config.trusted_rpc_node {
            Self::new_proven(query_client).await
        } else {
            Self::new_light_client(query_client, on_disk_checkpoint_file).await
        }
    }

    async fn new_proven(query_client: QueryHttpRpcNyxdClient) -> anyhow::Result<Self> {
        let directory_contract = query_client
            .directory_contract_address()
            .context("failed to get directory contract address")?;
        let anchor = ProvenTrustAnchor::new(
            query_client.clone_query_client(),
            directory_contract.clone(),
            digest_item_key(),
        );
        // we trust our rpc node, so we can get the chain id from it
        let chain_id = query_client.latest_block().await?.block.header.chain_id;

        Ok(ChainAnchor {
            anchor: Arc::new(anchor),
            chain_id,
            query_client,
        })
    }

    async fn new_light_client(
        query_client: QueryHttpRpcNyxdClient,
        on_disk_checkpoint_file: &Path,
    ) -> anyhow::Result<Self> {
        // seed a light client from a self-authenticating checkpoint, then advance it forward.
        // the loader tries the locally persisted head first, then the compiled-in / well-known
        // seed; the proven-RPC path above remains the default (`trusted_rpc_node`).
        let directory_contract = query_client
            .directory_contract_address()
            .context("failed to get directory contract address")?;

        // the light-client-verified head is persisted in the node data dir, so a restart can
        // reseed from it without a fresh checkpoint
        let store = FileCheckpointStore::new(on_disk_checkpoint_file);

        let checkpoint = get_trusted_checkpoint(&store).await?;
        let chain_id = checkpoint.signed_header.header.chain_id.clone();

        let anchor = LightClientAnchor::new(
            query_client.clone_query_client(),
            directory_contract.clone(),
            digest_item_key(),
            checkpoint,
            nyx_default_options(),
        )
        .with_store(store);

        Ok(ChainAnchor {
            anchor: Arc::new(anchor),
            chain_id,
            query_client,
        })
    }

    pub(crate) fn chain_id(&self) -> &chain::Id {
        &self.chain_id
    }

    /// The anchor for the contract at `contract`, whose digest is stored under the bare item key
    /// `digest_key` (the anchor prefixes the contract itself).
    pub(crate) fn contract_anchor(
        &self,
        contract: AccountId,
        digest_key: Vec<u8>,
    ) -> ContractAnchor {
        ContractAnchor {
            chain: Arc::clone(&self.anchor),
            query_client: self.query_client.clone_query_client(),
            contract,
            digest_key,
        }
    }
}

/// One contract's anchor: the chain anchor's app hash, and the contract's digest proven against it.
pub(crate) struct ContractAnchor {
    chain: Arc<dyn TrustAnchor + Send + Sync>,

    query_client: QueryHttpRpcNyxdClient,

    contract: AccountId,

    digest_key: Vec<u8>,
}

#[async_trait]
impl TrustAnchor for ContractAnchor {
    async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, AnchorError> {
        self.chain.trusted_app_hash(height).await
    }

    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, AnchorError> {
        let app_hash = self.trusted_app_hash(height).await?;
        get_trusted_digest(
            &self.query_client,
            &self.contract,
            &self.digest_key,
            height,
            app_hash,
        )
        .await
    }
}
