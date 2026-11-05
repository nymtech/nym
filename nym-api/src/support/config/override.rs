// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::cli::{init, run};
use std::{net::SocketAddr, time::Duration};

// Configuration that can be overridden.
pub(crate) struct OverrideConfig {
    /// Endpoint to nyxd instance used for contract information.
    pub(crate) nyxd_validator: Option<url::Url>,

    /// Bearer token for exposing and accessing additional utility routes
    pub(crate) utility_routes_bearer: Option<String>,

    /// Mnemonic used for the ecash signer's DKG and multisig transactions
    pub(crate) mnemonic: Option<bip39::Mnemonic>,

    /// Flag to indicate whether coconut signer authority is enabled on this API
    pub(crate) enable_zk_nym: Option<bool>,

    /// Announced address that is going to be put in the DKG contract where zk-nym clients will connect
    /// to obtain their credentials
    pub(crate) announce_address: Option<url::Url>,

    /// Socket address this api will use for binding its http API.
    /// default: `127.0.0.1:8080` in `debug` builds and `0.0.0.0:8080` in `release`
    pub(crate) bind_address: Option<SocketAddr>,

    pub(crate) address_cache_ttl: Option<Duration>,

    pub(crate) address_cache_capacity: Option<u64>,

    pub(crate) allow_illegal_ips: bool,
}

impl From<init::Args> for OverrideConfig {
    fn from(args: init::Args) -> Self {
        OverrideConfig {
            nyxd_validator: args.nyxd_validator,
            utility_routes_bearer: args.utility_routes_bearer,
            mnemonic: args.mnemonic,
            enable_zk_nym: Some(args.enable_zk_nym),
            announce_address: args.announce_address,
            bind_address: args.bind_address,
            // irrelevant for --init command because we set the value in --run
            address_cache_ttl: None,
            address_cache_capacity: None,
            allow_illegal_ips: args.allow_illegal_ips,
        }
    }
}

impl From<run::Args> for OverrideConfig {
    fn from(args: run::Args) -> Self {
        OverrideConfig {
            nyxd_validator: args.nyxd_validator,
            utility_routes_bearer: args.utility_routes_bearer,
            mnemonic: args.mnemonic,
            enable_zk_nym: args.enable_zk_nym,
            announce_address: args.announce_address,
            bind_address: args.bind_address,
            address_cache_ttl: args.address_cache_ttl_seconds.map(Duration::from_secs),
            address_cache_capacity: args.address_cache_capacity,
            allow_illegal_ips: args.allow_illegal_ips,
        }
    }
}
