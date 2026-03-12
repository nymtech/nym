// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub const LOCALNET_NYXD_IMAGE_NAME: &str = "localnet-nyxd";
pub const LOCALNET_NYM_BINARIES_IMAGE_NAME: &str = "localnet-nym-binaries";

pub const LOCALNET_NYXD_CONTAINER_NAME_SUFFIX: &str = "localnet-nyxd";
pub const LOCALNET_NYM_API_CONTAINER_NAME_SUFFIX: &str = "localnet-nym-api";
pub const LOCALNET_NYM_NODE_CONTAINER_NAME_SUFFIX: &str = "localnet-nym-node";

pub const NYM_NODE_HTTP_BEARER: &str = "dQw4w9WgXcQ";
pub const NYM_API_UTILITY_BEARER: &str = "dQw4w9WgXcQ";

pub const CONTAINER_NETWORK_NAME: &str = "nym-localnet";

// this value is quite arbitrary
pub const MIN_MASTER_UNYM_BALANCE: u128 = 10_000_000_000;

pub const CI_BUILD_SERVER: &str = "https://builds.ci.nymte.ch";

pub const CARGO_REGISTRY_CACHE_VOLUME: &str = "registry_cache";
pub const CONTRACTS_CACHE_VOLUME: &str = "nym_contracts_cache";

// filenames as created by our build pipeline as of 24.11.25
pub mod contract_build_names {
    pub const MULTISIG: &str = "cw3_flex_multisig.wasm";
    pub const GROUP: &str = "cw4_group.wasm";
    pub const MIXNET: &str = "mixnet_contract.wasm";
    pub const VESTING: &str = "vesting_contract.wasm";
    pub const DKG: &str = "nym_coconut_dkg.wasm";
    pub const ECASH: &str = "nym_ecash.wasm";
    pub const PERFORMANCE: &str = "nym_performance_contract.wasm";
    pub const NYM_POOL: &str = "nym_pool_contract.wasm";

    pub const DKG_BYPASS_CONTRACT: &str = "dkg_bypass_contract.wasm";
}
