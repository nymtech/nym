// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::try_load_current_config;
use crate::config::Config;
use crate::{
    commands::{override_config, OverrideConfig},
    error::Socks5ClientError,
};
use clap::Args;
use log::*;
use nym_bin_common::version_checker::is_minor_version_compatible;
use nym_client_core::client::base_client::storage::OnDiskPersistent;
use nym_crypto::asymmetric::identity;
use nym_socks5_client_core::NymClient;
use nym_sphinx::addressing::clients::Recipient;

#[derive(Args, Clone)]
pub(crate) struct Run {
    /// Id of the nym-mixnet-client we want to run.
    #[clap(long)]
    id: String,

    /// Specifies whether this client is going to use an anonymous sender tag for communication with the service provider.
    /// While this is going to hide its actual address information, it will make the actual communication
    /// slower and consume nearly double the bandwidth as it will require sending reply SURBs.
    ///
    /// Note that some service providers might not support this.
    // the alias here is included for backwards compatibility (1.1.4 and before)
    #[clap(long, alias = "use_anonymous_sender_tag")]
    use_anonymous_replies: Option<bool>,

    /// Address of the socks5 provider to send messages to.
    #[clap(long)]
    provider: Option<Recipient>,

    /// Id of the gateway we want to connect to. If overridden, it is user's responsibility to
    /// ensure prior registration happened
    #[clap(long)]
    gateway: Option<identity::PublicKey>,

    /// Comma separated list of rest endpoints of the nyxd validators
    #[clap(long, alias = "nyxd_validators", value_delimiter = ',', hide = true)]
    nyxd_urls: Option<Vec<url::Url>>,

    /// Comma separated list of rest endpoints of the Nym APIs
    #[clap(long, value_delimiter = ',')]
    nym_apis: Option<Vec<url::Url>>,

    /// Port for the socket to listen on
    #[clap(short, long)]
    port: Option<u16>,

    /// Mostly debug-related option to increase default traffic rate so that you would not need to
    /// modify config post init
    #[clap(long, hide = true)]
    fastmode: bool,

    /// Disable loop cover traffic and the Poisson rate limiter (for debugging only)
    #[clap(long, hide = true)]
    no_cover: bool,

    /// Enable medium mixnet traffic, for experiments only.
    /// This includes things like disabling cover traffic, no per hop delays, etc.
    #[clap(long, hide = true)]
    medium_toggle: bool,

    /// Set this client to work in a enabled credentials mode that would attempt to use gateway
    /// with bandwidth credential requirement.
    #[clap(long, hide = true)]
    enabled_credentials_mode: Option<bool>,

    #[clap(long, hide = true, action)]
    outfox: bool,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            nym_apis: run_config.nym_apis,
            port: run_config.port,
            use_anonymous_replies: run_config.use_anonymous_replies,
            fastmode: run_config.fastmode,
            no_cover: run_config.no_cover,
            medium_toggle: run_config.medium_toggle,
            nyxd_urls: run_config.nyxd_urls,
            enabled_credentials_mode: run_config.enabled_credentials_mode,
            outfox: run_config.outfox,
        }
    }
}

// this only checks compatibility between config the binary. It does not take into consideration
// network version. It might do so in the future.
fn version_check(cfg: &Config) -> bool {
    let binary_version = env!("CARGO_PKG_VERSION");
    let config_version = &cfg.core.base.client.version;
    if binary_version == config_version {
        true
    } else {
        warn!(
            "The socks5-client binary has different version than what is specified in config file! {binary_version} and {config_version}",
        );
        if is_minor_version_compatible(binary_version, config_version) {
            info!("but they are still semver compatible. However, consider running the `upgrade` command");
            true
        } else {
            error!("and they are semver incompatible! - please run the `upgrade` command before attempting `run` again");
            false
        }
    }
}

pub(crate) async fn execute(args: &Run) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    eprintln!("Starting client {}...", args.id);

    let mut config = try_load_current_config(&args.id)?;
    config = override_config(config, OverrideConfig::from(args.clone()));

    if !version_check(&config) {
        error!("failed the local version check");
        return Err(Box::new(Socks5ClientError::FailedLocalVersionCheck));
    }

    let storage =
        OnDiskPersistent::from_paths(config.storage_paths.common_paths, &config.core.base.debug)
            .await?;
    NymClient::new(config.core, storage).run_forever().await
}
