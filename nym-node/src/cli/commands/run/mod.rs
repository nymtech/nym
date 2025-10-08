// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::upgrade_helpers::try_load_current_config;
use crate::error::NymNodeError;
use crate::node::NymNode;
use crate::node::bonding_information::BondingInformation;
use crate::node::mixnet::packet_forwarding::global::is_global_ip;
use std::fs;
use std::net::IpAddr;
use tracing::{debug, info, trace, warn};

mod args;

pub(crate) use args::Args;

fn check_public_ips(ips: &[IpAddr], local: bool) -> Result<(), NymNodeError> {
    let mut suspicious_ip = Vec::new();
    for ip in ips {
        if !is_global_ip(ip) {
            if !local {
                return Err(NymNodeError::InvalidPublicIp { address: *ip });
            }
            suspicious_ip.push(ip);
        }
    }

    if !suspicious_ip.is_empty() {
        warn!("\n##### WARNING #####");
        for ip in suspicious_ip {
            warn!(
                "The 'public' IP address you're trying to announce: {ip} may not be accessible to other clients.\
            Please make sure this is what you intended to announce.\
            You can ignore this warning if you're running setup on a local network "
            )
        }
        warn!("\n##### WARNING #####\n");
    }
    Ok(())
}

pub(crate) async fn execute(mut args: Args) -> Result<(), NymNodeError> {
    trace!("passed arguments: {args:#?}");

    let config_path = args.config.config_path();
    let output = args.output;
    let bonding_info_path = args.bonding_information_output.clone();
    let init_only = args.init_only;
    let local = args.local;
    let accepted_operator_terms_and_conditions = args.accept_operator_terms_and_conditions;

    if !accepted_operator_terms_and_conditions {
        warn!("you don't seem to have accepted the terms and conditions of a Nym node operator");
        warn!(
            "please familiarise yourself with <https://nymtech.net/terms-and-conditions/operators/v1.0.0> and run the binary with '--accept-operator-terms-and-conditions' flag if you agree with them"
        );
    }

    let config = if !config_path.exists() {
        debug!("no configuration file found at '{}'", config_path.display());
        info!("initialising new nym-node");
        if args.deny_init {
            return Err(NymNodeError::ForbiddenInitialisation { config_path });
        }

        let maybe_custom_mnemonic = args.take_mnemonic();

        let config = args.build_config()?;
        NymNode::initialise(&config, maybe_custom_mnemonic).await?;

        config
    } else {
        info!(
            "attempting to load nym-node configuration from {}",
            config_path.display()
        );
        let write_changes = args.write_changes;
        let config = args.override_config(try_load_current_config(config_path).await?);

        if write_changes {
            config.save()?;
        }
        config
    };
    config.validate()?;

    if !config.modes.any_enabled() {
        warn!(
            "this node is going to run without mixnode or gateway support! consider providing `mode` value"
        );
    }

    if config.modes.standalone_exit() {
        warn!(
            "this node is going to run in EXIT gateway mode only - it will not be able to accept client traffic and thus will NOT be eligible for any rewards. consider running it alongside `entry` (or `full-gateway`) mode"
        )
    }

    if config.host.public_ips.is_empty() {
        return Err(NymNodeError::NoPublicIps);
    }
    check_public_ips(&config.host.public_ips, local)?;

    let mut config = config;
    if local {
        config.debug.testnet = true
    }

    let nym_node = NymNode::new(config)
        .await?
        .with_accepted_operator_terms_and_conditions(accepted_operator_terms_and_conditions);

    // if requested, write bonding info
    if let Some(bonding_info_path) = bonding_info_path {
        info!(
            "writing bonding information to '{}'",
            bonding_info_path.display()
        );
        let info =
            BondingInformation::from_data(nym_node.config(), *nym_node.ed25519_identity_key());
        let data = output.format(&info);
        fs::write(&bonding_info_path, data).map_err(|source| {
            NymNodeError::BondingInfoWriteFailure {
                path: bonding_info_path,
                source,
            }
        })?;
    }

    if init_only {
        debug!("returning due to the 'init-only' flag");
        return Ok(());
    }

    nym_node.run().await
}
