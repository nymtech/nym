// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::identity;
use std::path::PathBuf;

#[cfg_attr(feature = "cli", derive(clap::Args))]
#[derive(Debug, Clone)]
pub struct CommonClientRunArgs {
    /// Id of client we want to create config for.
    #[cfg_attr(feature = "cli", clap(long))]
    pub id: String,

    /// Id of the gateway we want to connect to. If overridden, it is user's responsibility to
    /// ensure prior registration happened
    #[cfg_attr(feature = "cli", clap(long))]
    pub gateway: Option<identity::PublicKey>,

    /// Comma separated list of rest endpoints of the nyxd validators
    #[cfg_attr(
        feature = "cli",
        clap(long, alias = "nyxd_validators", value_delimiter = ',', hide = true)
    )]
    pub nyxd_urls: Option<Vec<url::Url>>,

    /// Comma separated list of rest endpoints of the API validators
    #[cfg_attr(
        feature = "cli",
        clap(
            long,
            alias = "api_validators",
            value_delimiter = ',',
            group = "network"
        )
    )]
    pub nym_apis: Option<Vec<url::Url>>,

    ///Comma separated list of urls to use for domain fronting
    #[cfg_attr(
        feature = "cli",
        clap(long, value_delimiter = ',', requires = "nym_apis", hide = true)
    )]
    pub fronting_domains: Option<Vec<url::Url>>,

    /// Path to .json file containing custom network specification.
    #[cfg_attr(feature = "cli", clap(long, group = "network", hide = true))]
    pub custom_mixnet: Option<PathBuf>,

    /// Set this client to work in a enabled credentials mode that would attempt to use gateway
    /// with bandwidth credential requirement.
    #[cfg_attr(feature = "cli", clap(long, hide = true))]
    pub enabled_credentials_mode: Option<bool>,

    /// Mostly debug-related option to increase default traffic rate so that you would not need to
    /// modify config post init
    // note: we removed the 'conflicts_with = medium_toggle', but that's fine since NR
    // has defined the conflict on that field itself
    #[cfg_attr(feature = "cli", clap(long, hide = true))]
    pub fastmode: bool,

    /// Disable loop cover traffic and the Poisson rate limiter (for debugging only)
    // note: we removed the 'conflicts_with = medium_toggle', but that's fine since NR
    // has defined the conflict on that field itself
    #[cfg_attr(feature = "cli", clap(long, hide = true))]
    pub no_cover: bool,
}
