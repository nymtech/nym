// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::cli::{init, run};
use std::net::SocketAddr;

// Configuration that can be overridden.
pub(crate) struct OverrideConfig {
    /// Specifies whether network monitoring is enabled on this API
    pub(crate) enable_monitor: Option<bool>,

    /// Specifies whether network rewarding is enabled on this API
    pub(crate) enable_rewarding: Option<bool>,

    /// Endpoint to nyxd instance used for contract information.
    pub(crate) nyxd_validator: Option<url::Url>,

    /// Mnemonic of the network monitor used for sending rewarding and zk-nyms transactions
    pub(crate) mnemonic: Option<bip39::Mnemonic>,

    /// Flag to indicate whether coconut signer authority is enabled on this API
    pub(crate) enable_zk_nym: Option<bool>,

    /// Announced address that is going to be put in the DKG contract where zk-nym clients will connect
    /// to obtain their credentials
    pub(crate) announce_address: Option<url::Url>,

    /// Set this nym api to work in a enabled credentials that would attempt to use gateway with the bandwidth credential requirement
    pub(crate) monitor_credentials_mode: Option<bool>,

    /// Socket address this api will use for binding its http API.
    /// default: `127.0.0.1:8080` in `debug` builds and `0.0.0.0:8080` in `release`
    pub(crate) bind_address: Option<SocketAddr>,
}

impl From<init::Args> for OverrideConfig {
    fn from(args: init::Args) -> Self {
        OverrideConfig {
            enable_monitor: Some(args.enable_monitor),
            enable_rewarding: Some(args.enable_rewarding),
            nyxd_validator: args.nyxd_validator,
            mnemonic: args.mnemonic,
            enable_zk_nym: Some(args.enable_zk_nym),
            announce_address: args.announce_address,
            monitor_credentials_mode: Some(args.monitor_credentials_mode),
            bind_address: args.bind_address,
        }
    }
}

impl From<run::Args> for OverrideConfig {
    fn from(args: run::Args) -> Self {
        OverrideConfig {
            enable_monitor: args.enable_monitor,
            enable_rewarding: args.enable_rewarding,
            nyxd_validator: args.nyxd_validator,
            mnemonic: args.mnemonic,
            enable_zk_nym: args.enable_zk_nym,
            announce_address: args.announce_address,
            monitor_credentials_mode: args.monitor_credentials_mode,
            bind_address: args.bind_address,
        }
    }
}
