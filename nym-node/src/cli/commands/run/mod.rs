// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::bonding_information::BondingInformationV1;
use crate::node::NymNode;
use nym_config::helpers::SPECIAL_ADDRESSES;
use nym_node::config::upgrade_helpers::try_load_current_config;
use nym_node::error::NymNodeError;
use std::fs;
use std::net::IpAddr;
use tracing::log::warn;
use tracing::{debug, info, trace};

mod args;

pub(crate) use args::Args;

fn check_public_ips(ips: &[IpAddr], local: bool) -> Result<(), NymNodeError> {
    let mut suspicious_ip = Vec::new();
    for ip in ips {
        if SPECIAL_ADDRESSES.contains(ip) {
            if !local {
                return Err(NymNodeError::InvalidPublicIp { address: *ip });
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
        warn!("please familiarise yourself with <https://nymtech.net/terms-and-conditions/operators/v1.0.0> and run the binary with '--accept-toc' flag if you agree with them");
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

    if config.host.public_ips.is_empty() {
        return Err(NymNodeError::NoPublicIps);
    }
    check_public_ips(&config.host.public_ips, local)?;

    let nym_node = NymNode::new(config)
        .await?
        .with_accepted_operator_terms_and_conditions(accepted_operator_terms_and_conditions);

    // if requested, write bonding info
    if let Some(bonding_info_path) = bonding_info_path {
        info!(
            "writing bonding information to '{}'",
            bonding_info_path.display()
        );
        let info = BondingInformationV1::from_data(
            nym_node.mode(),
            nym_node.ed25519_identity_key().to_base58_string(),
            nym_node.x25519_sphinx_key().to_base58_string(),
        );
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
