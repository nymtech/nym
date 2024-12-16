// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::try_load_current_config;
use crate::commands::{override_config, OverrideConfig};
use clap::Args;
use nym_client_core::cli_helpers::client_run::CommonClientRunArgs;
use nym_client_core::client::base_client::storage::OnDiskPersistent;
use nym_client_core::client::topology_control::geo_aware_provider::CountryGroup;
use nym_socks5_client_core::NymClient;
use nym_sphinx::addressing::clients::Recipient;
use std::net::IpAddr;

#[derive(Args, Clone)]
pub(crate) struct Run {
    #[command(flatten)]
    common_args: CommonClientRunArgs,

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

    /// Port for the socket to listen on
    #[clap(short, long)]
    port: Option<u16>,

    /// The custom host on which the socks5 client will be listening for requests
    #[clap(long)]
    host: Option<IpAddr>,

    /// Set geo-aware mixnode selection when sending mixnet traffic, for experiments only.
    #[clap(long, hide = true, value_parser = validate_country_group, group="routing")]
    geo_routing: Option<CountryGroup>,

    /// Enable medium mixnet traffic, for experiments only.
    /// This includes things like disabling cover traffic, no per hop delays, etc.
    #[clap(long, hide = true)]
    medium_toggle: bool,

    #[clap(long, hide = true, action)]
    outfox: bool,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            nym_apis: run_config.common_args.nym_apis,
            ip: run_config.host,
            port: run_config.port,
            use_anonymous_replies: run_config.use_anonymous_replies,
            fastmode: run_config.common_args.fastmode,
            no_cover: run_config.common_args.no_cover,
            geo_routing: run_config.geo_routing,
            medium_toggle: run_config.medium_toggle,
            nyxd_urls: run_config.common_args.nyxd_urls,
            enabled_credentials_mode: run_config.common_args.enabled_credentials_mode,
            outfox: run_config.outfox,
            stats_reporting_address: run_config.common_args.stats_reporting_address,
        }
    }
}

fn validate_country_group(s: &str) -> Result<CountryGroup, String> {
    match s.parse() {
        Ok(cg) => Ok(cg),
        Err(_) => Err(format!("failed to parse country group: {}", s)),
    }
}

pub(crate) async fn execute(args: Run) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    eprintln!("Starting client {}...", args.common_args.id);

    let mut config = try_load_current_config(&args.common_args.id).await?;
    config = override_config(config, OverrideConfig::from(args.clone()));

    let storage =
        OnDiskPersistent::from_paths(config.storage_paths.common_paths, &config.core.base.debug)
            .await?;
    let user_agent = nym_bin_common::bin_info!().into();
    NymClient::new(
        config.core,
        storage,
        user_agent,
        args.common_args.custom_mixnet,
    )
    .run_forever()
    .await
}
