// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash::models::EcashSignerStatusResponse;
use crate::models::tendermint_types::{BlockHeader, BlockId};
use crate::models::{ChainStatus, SignerInformationResponse};
use crate::signable::SignedMessage;
use nym_coconut_dkg_common::types::EpochId;
use nym_crypto::asymmetric::ed25519::PublicKey;
use nym_ecash_signer_check_types::helper_traits::{
    ChainResponse, LegacyChainResponse, LegacySignerResponse, SignerResponse, TimestampedResponse,
    Verifiable,
};
use nym_ecash_signer_check_types::status::SignerResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::OffsetDateTime;
use utoipa::ToSchema;

pub type ChainBlocksStatusResponse = SignedMessage<ChainBlocksStatusResponseBody>;
pub type SignersStatusResponse = SignedMessage<SignersStatusResponseBody>;
pub type DetailedSignersStatusResponse = SignedMessage<DetailedSignersStatusResponseBody>;

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignersStatusResponseBody {
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String)]
    pub as_at: OffsetDateTime,

    pub overview: SignersStatusOverview,

    pub results: Vec<MinimalSignerResult>,
}

pub type TypedSignerResult = SignerResult<
    SignerInformationResponse,
    EcashSignerStatusResponse,
    ChainStatusResponse,
    ChainBlocksStatusResponse,
>;

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MinimalSignerResult {
    pub announce_address: String,
    pub owner_address: String,
    pub node_index: u64,
    pub public_key: String,

    pub local_chain_working: bool,
    pub credential_issuance_available: bool,
}

impl From<&TypedSignerResult> for MinimalSignerResult {
    fn from(result: &TypedSignerResult) -> MinimalSignerResult {
        MinimalSignerResult {
            announce_address: result.information.announce_address.clone(),
            owner_address: result.information.owner_address.clone(),
            node_index: result.information.node_index,
            public_key: result.information.public_key.clone(),
            local_chain_working: result.chain_available(),
            credential_issuance_available: result.signing_available(),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DetailedSignersStatusResponseBody {
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String)]
    pub as_at: OffsetDateTime,

    pub overview: SignersStatusOverview,

    pub details: Vec<TypedSignerResult>,
}

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignersStatusOverview {
    #[schema(value_type = Option<u64>)]
    pub epoch_id: Option<EpochId>,

    pub signing_threshold: Option<u64>,
    pub threshold_available: Option<bool>,

    pub total_signers: usize,
    pub unreachable_signers: usize,
    pub malformed_signers: usize,

    // unreachable or outdated
    pub unknown_local_chain_status: usize,
    pub working_local_chain: usize,

    // i.e. provided signature
    pub provably_stalled_local_chain: usize,
    pub unprovably_stalled_local_chain: usize,

    // unreachable or outdated
    pub unknown_credential_issuance_status: usize,
    pub working_credential_issuance: usize,

    // i.e. provided signature
    pub provably_unavailable_credential_issuance: usize,
    pub unprovably_unavailable_credential_issuance: usize,
}

impl SignersStatusOverview {
    pub fn new(results: &[TypedSignerResult], signing_threshold: Option<u64>) -> Self {
        let epoch_id = results.first().map(|r| r.dkg_epoch_id);

        let mut unreachable_signers = 0;
        let mut malformed_signers = 0;
        let mut unknown_local_chain_status = 0;
        let mut working_local_chain = 0;
        let mut provably_stalled_local_chain = 0;
        let mut unprovably_stalled_local_chain = 0;
        let mut unknown_credential_issuance_status = 0;
        let mut working_credential_issuance = 0;
        let mut provably_unavailable_credential_issuance = 0;
        let mut unprovably_unavailable_credential_issuance = 0;

        for result in results {
            if result.signer_unreachable() {
                unreachable_signers += 1;
            }
            if result.malformed_details() {
                malformed_signers += 1;
            }

            if result.unknown_chain_status() {
                unknown_local_chain_status += 1;
            }
            if result.chain_available() {
                working_local_chain += 1;
            }
            if result.chain_provably_stalled() {
                provably_stalled_local_chain += 1;
            }
            if result.chain_unprovably_stalled() {
                unprovably_stalled_local_chain += 1;
            }

            if result.unknown_signing_status() {
                unknown_credential_issuance_status += 1;
            }
            if result.signing_available() {
                working_credential_issuance += 1;
            }
            if result.signing_provably_unavailable() {
                provably_unavailable_credential_issuance += 1;
            }
            if result.signing_unprovably_unavailable() {
                unprovably_unavailable_credential_issuance += 1;
            }
        }

        SignersStatusOverview {
            epoch_id,
            signing_threshold,
            threshold_available: signing_threshold.map(|threshold| {
                (working_local_chain as u64) >= threshold
                    && (working_credential_issuance as u64) >= threshold
            }),
            total_signers: results.len(),
            unreachable_signers,
            malformed_signers,
            unknown_local_chain_status,
            working_local_chain,
            provably_stalled_local_chain,
            unprovably_stalled_local_chain,
            unknown_credential_issuance_status,
            working_credential_issuance,
            provably_unavailable_credential_issuance,
            unprovably_unavailable_credential_issuance,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChainBlocksStatusResponseBody {
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String)]
    pub current_time: OffsetDateTime,

    pub latest_cached_block: Option<DetailedChainStatus>,

    // explicit indication of THIS signer whether it thinks the chain is stalled
    pub chain_status: ChainStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ChainStatusResponse {
    pub connected_nyxd: String,
    pub status: DetailedChainStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct DetailedChainStatus {
    pub abci: crate::models::tendermint_types::AbciInfo,
    pub latest_block: BlockInfo,
}

impl DetailedChainStatus {
    pub fn stall_status(&self, now: OffsetDateTime, threshold: Duration) -> ChainStatus {
        let block_time: OffsetDateTime = self.latest_block.block.header.time.into();
        let diff = now - block_time;
        if diff > threshold {
            ChainStatus::Stalled {
                approximate_amount: diff.unsigned_abs(),
            }
        } else {
            ChainStatus::Synced
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BlockInfo {
    pub block_id: BlockId,
    pub block: FullBlockInfo,
    // if necessary we might put block data here later too
}

impl From<tendermint_rpc::endpoint::block::Response> for BlockInfo {
    fn from(value: tendermint_rpc::endpoint::block::Response) -> Self {
        BlockInfo {
            block_id: value.block_id.into(),
            block: FullBlockInfo {
                header: value.block.header.into(),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct FullBlockInfo {
    pub header: BlockHeader,
}

// copy tendermint types definitions whilst deriving schema types on them and dropping unwanted fields
pub mod tendermint_types {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use tendermint::abci::response::Info;
    use tendermint::block::header::Version;
    use tendermint::{block, Hash};
    use utoipa::ToSchema;

    #[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
    pub struct AbciInfo {
        /// Some arbitrary information.
        pub data: String,

        /// The application software semantic version.
        pub version: String,

        /// The application protocol version.
        pub app_version: u64,

        /// The latest block for which the app has called [`Commit`].
        pub last_block_height: u64,

        /// The latest result of [`Commit`].
        pub last_block_app_hash: String,
    }

    impl From<Info> for AbciInfo {
        fn from(value: Info) -> Self {
            AbciInfo {
                data: value.data,
                version: value.version,
                app_version: value.app_version,
                last_block_height: value.last_block_height.value(),
                last_block_app_hash: value.last_block_app_hash.to_string(),
            }
        }
    }

    /// `Version` contains the protocol version for the blockchain and the
    /// application.
    ///
    /// <https://github.com/tendermint/spec/blob/d46cd7f573a2c6a2399fcab2cde981330aa63f37/spec/core/data_structures.md#version>
    #[derive(
        Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, ToSchema,
    )]
    pub struct HeaderVersion {
        /// Block version
        pub block: u64,

        /// App version
        pub app: u64,
    }

    impl From<tendermint::block::header::Version> for HeaderVersion {
        fn from(value: Version) -> Self {
            HeaderVersion {
                block: value.block,
                app: value.app,
            }
        }
    }

    /// Block identifiers which contain two distinct Merkle roots of the block,
    /// as well as the number of parts in the block.
    ///
    /// <https://github.com/tendermint/spec/blob/d46cd7f573a2c6a2399fcab2cde981330aa63f37/spec/core/data_structures.md#blockid>
    ///
    /// Default implementation is an empty Id as defined by the Go implementation in
    /// <https://github.com/tendermint/tendermint/blob/1635d1339c73ae6a82e062cd2dc7191b029efa14/types/block.go#L1204>.
    ///
    /// If the Hash is empty in BlockId, the BlockId should be empty (encoded to None).
    /// This is implemented outside of this struct. Use the Default trait to check for an empty BlockId.
    /// See: <https://github.com/informalsystems/tendermint-rs/issues/663>
    #[derive(
        Serialize,
        Deserialize,
        Copy,
        Clone,
        Debug,
        Default,
        Hash,
        Eq,
        PartialEq,
        PartialOrd,
        Ord,
        JsonSchema,
        ToSchema,
    )]
    pub struct BlockId {
        /// The block's main hash is the Merkle root of all the fields in the
        /// block header.
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub hash: Hash,

        /// Parts header (if available) is used for secure gossipping of the block
        /// during consensus. It is the Merkle root of the complete serialized block
        /// cut into parts.
        ///
        /// PartSet is used to split a byteslice of data into parts (pieces) for
        /// transmission. By splitting data into smaller parts and computing a
        /// Merkle root hash on the list, you can verify that a part is
        /// legitimately part of the complete data, and the part can be forwarded
        /// to other peers before all the parts are known. In short, it's a fast
        /// way to propagate a large file over a gossip network.
        ///
        /// <https://github.com/tendermint/tendermint/wiki/Block-Structure#partset>
        ///
        /// PartSetHeader in protobuf is defined as never nil using the gogoproto
        /// annotations. This does not translate to Rust, but we can indicate this
        /// in the domain type.
        pub part_set_header: PartSetHeader,
    }

    impl From<block::Id> for BlockId {
        fn from(value: block::Id) -> Self {
            BlockId {
                hash: value.hash,
                part_set_header: value.part_set_header.into(),
            }
        }
    }

    /// Block parts header
    #[derive(
        Clone,
        Copy,
        Debug,
        Default,
        Hash,
        Eq,
        PartialEq,
        PartialOrd,
        Ord,
        Deserialize,
        Serialize,
        JsonSchema,
        ToSchema,
    )]
    #[non_exhaustive]
    pub struct PartSetHeader {
        /// Number of parts in this block
        pub total: u32,

        /// Hash of the parts set header,
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub hash: Hash,
    }

    impl From<tendermint::block::parts::Header> for PartSetHeader {
        fn from(value: block::parts::Header) -> Self {
            PartSetHeader {
                total: value.total,
                hash: value.hash,
            }
        }
    }

    /// Block `Header` values contain metadata about the block and about the
    /// consensus, as well as commitments to the data in the current block, the
    /// previous block, and the results returned by the application.
    ///
    /// <https://github.com/tendermint/spec/blob/d46cd7f573a2c6a2399fcab2cde981330aa63f37/spec/core/data_structures.md#header>
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
    pub struct BlockHeader {
        /// Header version
        pub version: HeaderVersion,

        /// Chain ID
        pub chain_id: String,

        /// Current block height
        pub height: u64,

        /// Current timestamp
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub time: tendermint::Time,

        /// Previous block info
        pub last_block_id: Option<BlockId>,

        /// Commit from validators from the last block
        #[schemars(with = "Option<String>")]
        #[schema(value_type = Option<String>)]
        pub last_commit_hash: Option<Hash>,

        /// Merkle root of transaction hashes
        #[schemars(with = "Option<String>")]
        #[schema(value_type = Option<String>)]
        pub data_hash: Option<Hash>,

        /// Validators for the current block
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub validators_hash: Hash,

        /// Validators for the next block
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub next_validators_hash: Hash,

        /// Consensus params for the current block
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub consensus_hash: Hash,

        /// State after txs from the previous block
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub app_hash: Hash,

        /// Root hash of all results from the txs from the previous block
        #[schemars(with = "Option<String>")]
        #[schema(value_type = Option<String>)]
        pub last_results_hash: Option<Hash>,

        /// Hash of evidence included in the block
        #[schemars(with = "Option<String>")]
        #[schema(value_type = Option<String>)]
        pub evidence_hash: Option<Hash>,

        /// Original proposer of the block
        #[serde(with = "nym_serde_helpers::hex")]
        #[schemars(with = "String")]
        #[schema(value_type = String)]
        pub proposer_address: Vec<u8>,
    }

    impl From<block::Header> for BlockHeader {
        fn from(value: block::Header) -> Self {
            BlockHeader {
                version: value.version.into(),
                chain_id: value.chain_id.to_string(),
                height: value.height.value(),
                time: value.time,
                last_block_id: value.last_block_id.map(Into::into),
                last_commit_hash: value.last_commit_hash,
                data_hash: value.data_hash,
                validators_hash: value.validators_hash,
                next_validators_hash: value.next_validators_hash,
                consensus_hash: value.consensus_hash,
                app_hash: Hash::try_from(value.app_hash.as_bytes().to_vec()).unwrap_or_default(),
                last_results_hash: value.last_results_hash,
                evidence_hash: value.evidence_hash,
                proposer_address: value.proposer_address.as_bytes().to_vec(),
            }
        }
    }
}

//  implement required traits for the signer responses

impl LegacyChainResponse for ChainStatusResponse {
    fn chain_synced(&self, now: OffsetDateTime, stall_threshold: Duration) -> bool {
        self.status.stall_status(now, stall_threshold).is_synced()
    }
}

impl Verifiable for ChainBlocksStatusResponse {
    fn verify_signature(&self, pub_key: &PublicKey) -> bool {
        self.verify_signature(pub_key)
    }
}

impl TimestampedResponse for ChainBlocksStatusResponse {
    fn timestamp(&self) -> OffsetDateTime {
        self.body.current_time
    }
}

impl ChainResponse for ChainBlocksStatusResponse {
    fn chain_synced(&self) -> bool {
        self.body.chain_status.is_synced()
    }
}

impl LegacySignerResponse for SignerInformationResponse {
    fn signer_identity(&self) -> &str {
        &self.identity
    }

    fn signer_verification_key(&self) -> &Option<String> {
        &self.verification_key
    }
}

impl Verifiable for EcashSignerStatusResponse {
    fn verify_signature(&self, pub_key: &PublicKey) -> bool {
        self.verify_signature(pub_key)
    }
}

impl TimestampedResponse for EcashSignerStatusResponse {
    fn timestamp(&self) -> OffsetDateTime {
        self.body.current_time
    }
}

impl SignerResponse for EcashSignerStatusResponse {
    fn has_signing_keys(&self) -> bool {
        self.body.has_signing_keys
    }

    fn signer_disabled(&self) -> bool {
        self.body.signer_disabled
    }

    fn is_ecash_signer(&self) -> bool {
        self.body.is_ecash_signer
    }

    fn dkg_ecash_epoch_id(&self) -> EpochId {
        self.body.dkg_ecash_epoch_id
    }
}
