// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::commands::helpers::{try_load_current_config, try_override_config, OverrideConfig};
use anyhow::bail;
use clap::Args;
use log::warn;
use nym_bin_common::output_format::OutputFormat;
use nym_config::helpers::SPECIAL_ADDRESSES;
use nym_gateway::helpers::{OverrideIpPacketRouterConfig, OverrideNetworkRequesterConfig};
use nym_gateway::GatewayError;
use std::net::IpAddr;
use std::path::PathBuf;

#[derive(Args, Clone)]
pub struct Run {
    /// Id of the gateway we want to run
    #[arg(long)]
    id: String,

    /// The custom listening address on which the gateway will be running for receiving sphinx packets
    #[arg(long, alias = "host")]
    listening_address: Option<IpAddr>,

    /// Comma separated list of public ip addresses that will be announced to the nym-api and subsequently to the clients.
    /// In nearly all circumstances, it's going to be identical to the address you're going to use for bonding.
    #[arg(long, value_delimiter = ',')]
    public_ips: Option<Vec<IpAddr>>,

    /// Optional hostname associated with this gateway that will be announced to the nym-api and subsequently to the clients
    #[arg(long)]
    hostname: Option<String>,

    /// The port on which the gateway will be listening for sphinx packets
    #[arg(long)]
    mix_port: Option<u16>,

    /// The port on which the gateway will be listening for clients gateway-requests
    #[arg(long)]
    clients_port: Option<u16>,

    /// Path to sqlite database containing all gateway persistent data
    #[arg(long)]
    datastore: Option<PathBuf>,

    /// Comma separated list of endpoints of nym APIs
    #[arg(
        long,
        alias = "validator_apis",
        value_delimiter = ',',
        group = "network"
    )]
    // the alias here is included for backwards compatibility (1.1.4 and before)
    nym_apis: Option<Vec<url::Url>>,

    /// Comma separated list of endpoints of the validator
    #[arg(
        long,
        alias = "validators",
        alias = "nyxd_validators",
        value_delimiter = ',',
        hide = true
    )]
    // the alias here is included for backwards compatibility (1.1.4 and before)
    nyxd_urls: Option<Vec<url::Url>>,

    /// Cosmos wallet mnemonic
    #[arg(long)]
    mnemonic: Option<bip39::Mnemonic>,

    /// Set this gateway to work only with coconut credentials; that would disallow clients to
    /// bypass bandwidth credential requirement
    #[arg(long, hide = true)]
    only_coconut_credentials: Option<bool>,

    /// Enable/disable gateway anonymized statistics that get sent to a statistics aggregator server
    #[arg(long)]
    enabled_statistics: Option<bool>,

    /// URL where a statistics aggregator is running. The default value is a Nym aggregator server
    #[arg(long)]
    statistics_service_url: Option<url::Url>,

    /// Allows this gateway to run an embedded network requester for minimal network overhead
    #[arg(long)]
    with_network_requester: Option<bool>,

    /// Allows this gateway to run an embedded network requester for minimal network overhead
    #[arg(long, hide = true)]
    with_ip_packet_router: Option<bool>,

    // ##### NETWORK REQUESTER FLAGS #####
    /// Specifies whether this network requester should run in 'open-proxy' mode
    #[arg(long)]
    open_proxy: Option<bool>,

    /// Enable service anonymized statistics that get sent to a statistics aggregator server
    #[arg(long)]
    enable_statistics: Option<bool>,

    /// Mixnet client address where a statistics aggregator is running. The default value is a Nym
    /// aggregator client
    #[arg(long)]
    statistics_recipient: Option<String>,

    /// Mostly debug-related option to increase default traffic rate so that you would not need to
    /// modify config post init
    #[arg(long, hide = true, conflicts_with = "medium_toggle")]
    fastmode: bool,

    /// Disable loop cover traffic and the Poisson rate limiter (for debugging only)
    #[arg(long, hide = true, conflicts_with = "medium_toggle")]
    no_cover: bool,

    /// Enable medium mixnet traffic, for experiments only.
    /// This includes things like disabling cover traffic, no per hop delays, etc.
    #[arg(
        long,
        hide = true,
        conflicts_with = "no_cover",
        conflicts_with = "fastmode"
    )]
    medium_toggle: bool,

    /// Path to .json file containing custom network specification.
    /// Only usable when local network requester is enabled.
    #[arg(long, group = "network", hide = true)]
    custom_mixnet: Option<PathBuf>,

    #[arg(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,

    /// Flag specifying this node will be running in a local setting.
    #[arg(long)]
    local: bool,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            listening_address: run_config.listening_address,
            public_ips: run_config.public_ips,
            hostname: run_config.hostname,
            mix_port: run_config.mix_port,
            clients_port: run_config.clients_port,
            datastore: run_config.datastore,
            nym_apis: run_config.nym_apis,
            mnemonic: run_config.mnemonic,

            enabled_statistics: run_config.enabled_statistics,
            statistics_service_url: run_config.statistics_service_url,
            nyxd_urls: run_config.nyxd_urls,
            only_coconut_credentials: run_config.only_coconut_credentials,
            with_network_requester: run_config.with_network_requester,
            with_ip_packet_router: run_config.with_ip_packet_router,
        }
    }
}

impl<'a> From<&'a Run> for OverrideNetworkRequesterConfig {
    fn from(value: &'a Run) -> Self {
        OverrideNetworkRequesterConfig {
            fastmode: value.fastmode,
            no_cover: value.no_cover,
            medium_toggle: value.medium_toggle,
            open_proxy: value.open_proxy,
            enable_statistics: value.enable_statistics,
            statistics_recipient: value.statistics_recipient.clone(),
        }
    }
}

impl From<&Run> for OverrideIpPacketRouterConfig {
    fn from(_value: &Run) -> Self {
        OverrideIpPacketRouterConfig {}
    }
}

fn show_binding_warning(address: IpAddr) {
    eprintln!("\n##### NOTE #####");
    eprintln!(
        "\nYou are trying to bind to {address} - you might not be accessible to other nodes\n\
         You can ignore this warning if you're running setup on a local network \n\
         or have used different host when bonding your node"
    );
    eprintln!("\n\n");
}

fn check_public_ips(ips: &[IpAddr], local: bool) -> anyhow::Result<()> {
    let mut suspicious_ip = Vec::new();
    for ip in ips {
        if SPECIAL_ADDRESSES.contains(ip) {
            if !local {
                return Err(GatewayError::InvalidPublicIp { address: *ip }.into());
            }
            suspicious_ip.push(ip);
        }
    }

    if !suspicious_ip.is_empty() {
        warn!("\n##### WARNING #####");
        for ip in suspicious_ip {
            warn!("The 'public' IP address you're trying to announce: {ip} may not be accessible to other clients.\
            Please make sure this is what you intended to announce.\
            You can ignore this warning if you're running setup on a local network ")
        }
        warn!("\n##### WARNING #####\n");
    }
    Ok(())
}

pub async fn execute(args: Run) -> anyhow::Result<()> {
    let id = args.id.clone();
    let local = args.local;

    eprintln!("Starting gateway {id}...");

    let output = args.output;
    let custom_mixnet = args.custom_mixnet.clone();
    let nr_opts = (&args).into();
    let ip_opts = (&args).into();

    let mut config = try_load_current_config(&args.id)?;
    config = try_override_config(config, args)?;

    let public_ips = &config.host.public_ips;
    if public_ips.is_empty() {
        return Err(GatewayError::NoPublicIps.into());
    }
    check_public_ips(public_ips, local)?;
    if config.gateway.clients_wss_port.is_some() && config.host.hostname.is_none() {
        bail!("attempted to announce 'wss' port without a valid hostname")
    }

    if SPECIAL_ADDRESSES.contains(&config.gateway.listening_address) {
        show_binding_warning(config.gateway.listening_address);
    }

    let gateway =
        nym_gateway::create_gateway(config, Some(nr_opts), Some(ip_opts), custom_mixnet).await?;
    let node_details = gateway.node_details().await?;
    eprintln!(
        "\nTo bond your gateway you will need to install the Nym wallet, go to https://nymtech.net/get-involved and select the Download button.\n\
         Select the correct version and install it to your machine. You will need to provide some of the following: \n ");
    output.to_stdout(&node_details);

    gateway.run().await?;
    Ok(())
}
