// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network::models::tendermint_types::{AbciInfo, BlockHeader, BlockId};
use nym_config::defaults::NymNetworkDetails;
use nym_validator_client::nyxd::BlockResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct NetworkDetails {
    pub(crate) connected_nyxd: String,
    pub(crate) network: NymNetworkDetails,
}

impl NetworkDetails {
    pub fn new(connected_nyxd: String, network: NymNetworkDetails) -> Self {
        Self {
            connected_nyxd,
            network,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ContractInformation<T> {
    pub(crate) address: Option<String>,
    pub(crate) details: Option<T>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ChainStatusResponse {
    pub connected_nyxd: String,
    pub status: ChainStatus,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct ChainStatus {
    pub abci: AbciInfo,
    pub latest_block: BlockInfo,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct BlockInfo {
    pub block_id: BlockId,
    pub block: FullBlockInfo,
    // if necessary we might put block data here later too
}

impl From<BlockResponse> for BlockInfo {
    fn from(value: BlockResponse) -> Self {
        BlockInfo {
            block_id: value.block_id.into(),
            block: FullBlockInfo {
                header: value.block.header.into(),
            },
        }
    }
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct FullBlockInfo {
    pub header: BlockHeader,
}

// copy tendermint types definitions whilst deriving schema types on them and dropping unwanted fields
pub mod tendermint_types {
    use nym_validator_client::nyxd::Hash;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use tendermint::abci::response::Info;
    use tendermint::block;
    use tendermint::block::header::Version;
    use utoipa::ToSchema;

    #[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
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
