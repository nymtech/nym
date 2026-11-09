// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Trust in the Nyx chain, shared by every contract nym-api attests: one chain anchor (a proven
//! read through the trusted rpc node, or a light client seeded from a checkpoint) establishes the
//! app hash at a height, and each contract proves its own digest against it.

use crate::support::chain_anchor::checkpoint::get_trusted_checkpoint;
use crate::support::config::DirectoryConfigDebug;
use nym_contract_anchor::anchor::checkpoint::store::FileCheckpointStore;
use nym_contract_anchor::anchor::contract::ContractAnchor;
use nym_contract_anchor::anchor::proven::ProvenChainAnchor;
use nym_contract_anchor::anchor::{nyx_default_options, ChainAnchor, LightClientAnchor};
use nym_contract_anchor::error::AnchorError;
use nym_validator_client::nyxd::{AccountId, TendermintRpcClient};
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::path::Path;
use std::sync::Arc;
use tendermint::block::Height;
use tendermint::chain;
use tendermint::hash::AppHash;

mod checkpoint;

/// A contract's anchor over the shared chain anchor.
pub(crate) type SharedContractAnchor =
    ContractAnchor<Arc<dyn ChainAnchor + Send + Sync>, QueryHttpRpcNyxdClient>;

/// The chain anchor every attested contract's [`SharedContractAnchor`] takes its app hashes from.
pub(crate) struct SharedChainAnchor {
    anchor: Arc<dyn ChainAnchor + Send + Sync>,

    chain_id: chain::Id,

    query_client: QueryHttpRpcNyxdClient,
}

// derived `Clone` would ask for a `Clone` query client, which only has `clone_query_client`
impl Clone for SharedChainAnchor {
    fn clone(&self) -> Self {
        SharedChainAnchor {
            anchor: Arc::clone(&self.anchor),
            chain_id: self.chain_id.clone(),
            query_client: self.query_client.clone_query_client(),
        }
    }
}

impl SharedChainAnchor {
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
        let anchor = ProvenChainAnchor::new(query_client.clone_query_client());
        // we trust our rpc node, so we can get the chain id from it
        let chain_id = query_client.latest_block().await?.block.header.chain_id;

        Ok(SharedChainAnchor {
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

        // the light-client-verified head is persisted in the node data dir, so a restart can
        // reseed from it without a fresh checkpoint
        let store = FileCheckpointStore::new(on_disk_checkpoint_file);

        let checkpoint = get_trusted_checkpoint(&store).await?;
        let chain_id = checkpoint.signed_header.header.chain_id.clone();

        let anchor = LightClientAnchor::new(
            query_client.clone_query_client(),
            checkpoint,
            nyx_default_options(),
        )
        .with_store(store);

        Ok(SharedChainAnchor {
            anchor: Arc::new(anchor),
            chain_id,
            query_client,
        })
    }

    pub(crate) fn chain_id(&self) -> &chain::Id {
        &self.chain_id
    }

    pub(crate) fn query_client(&self) -> &QueryHttpRpcNyxdClient {
        &self.query_client
    }

    /// The app hash committing the chain's state at `height`, which every contract proves against.
    pub(crate) async fn trusted_app_hash(&self, height: Height) -> Result<AppHash, AnchorError> {
        self.anchor.trusted_app_hash(height).await
    }

    /// The anchor for the contract at `contract`, whose digest is stored under the bare item key
    /// `digest_key` (the anchor prefixes the contract itself).
    pub(crate) fn contract_anchor(
        &self,
        contract: AccountId,
        digest_key: Vec<u8>,
    ) -> SharedContractAnchor {
        ContractAnchor::new(
            Arc::clone(&self.anchor),
            self.query_client.clone_query_client(),
            contract,
            digest_key,
        )
    }
}
