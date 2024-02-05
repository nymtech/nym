// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::{debug, info};
use nym_coconut_dkg_common::dealing::DEFAULT_DEALINGS;
use std::str::FromStr;

use nym_coconut_dkg_common::msg::InstantiateMsg;
use nym_coconut_dkg_common::types::TimeConfiguration;
use nym_validator_client::nyxd::AccountId;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub group_addr: String,

    #[clap(long)]
    pub multisig_addr: Option<AccountId>,

    #[clap(long)]
    pub public_key_submission_time_secs: Option<u64>,

    #[clap(long)]
    pub dealing_exchange_time_secs: Option<u64>,

    #[clap(long)]
    pub verification_key_submission_time_secs: Option<u64>,

    #[clap(long)]
    pub verification_key_validation_time_secs: Option<u64>,

    #[clap(long)]
    pub verification_key_finalization_time_secs: Option<u64>,

    #[clap(long)]
    pub in_progress_time_secs: Option<u64>,

    #[clap(long)]
    pub mix_denom: Option<String>,
}

pub async fn generate(args: Args) {
    info!("Starting to generate vesting contract instantiate msg");

    debug!("Received arguments: {:?}", args);

    let multisig_addr = args.multisig_addr.unwrap_or_else(|| {
        let address = std::env::var(nym_network_defaults::var_names::REWARDING_VALIDATOR_ADDRESS)
            .expect("Multisig address has to be set");
        AccountId::from_str(address.as_str())
            .expect("Failed converting multisig address to AccountId")
    });

    let mix_denom = args.mix_denom.unwrap_or_else(|| {
        std::env::var(nym_network_defaults::var_names::MIX_DENOM).expect("Mix denom has to be set")
    });

    let mut time_configuration = if let Ok(config) =
        std::env::var(nym_network_defaults::var_names::DKG_TIME_CONFIGURATION)
    {
        TimeConfiguration::from_str(&config).expect("Invalid env variable value")
    } else {
        TimeConfiguration::default()
    };
    if let Some(public_key_submission_time_secs) = args.public_key_submission_time_secs {
        time_configuration.public_key_submission_time_secs = public_key_submission_time_secs;
    }
    if let Some(dealing_exchange_time_secs) = args.dealing_exchange_time_secs {
        time_configuration.dealing_exchange_time_secs = dealing_exchange_time_secs;
    }
    if let Some(verification_key_submission_time_secs) = args.verification_key_submission_time_secs
    {
        time_configuration.verification_key_submission_time_secs =
            verification_key_submission_time_secs;
    }
    if let Some(verification_key_validation_time_secs) = args.verification_key_validation_time_secs
    {
        time_configuration.verification_key_validation_time_secs =
            verification_key_validation_time_secs;
    }
    if let Some(verification_key_finalization_time_secs) =
        args.verification_key_finalization_time_secs
    {
        time_configuration.verification_key_finalization_time_secs =
            verification_key_finalization_time_secs;
    }
    if let Some(in_progress_time_secs) = args.in_progress_time_secs {
        time_configuration.in_progress_time_secs = in_progress_time_secs;
    }

    let instantiate_msg = InstantiateMsg {
        group_addr: args.group_addr,
        multisig_addr: multisig_addr.to_string(),
        time_configuration: Some(time_configuration),
        mix_denom,
        key_size: DEFAULT_DEALINGS as u32,
    };

    debug!("instantiate_msg: {:?}", instantiate_msg);

    let res =
        serde_json::to_string(&instantiate_msg).expect("failed to convert instantiate msg to json");

    println!("{res}")
}
