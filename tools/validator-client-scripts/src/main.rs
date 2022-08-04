// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::bond_gateway::bond_gateway;
use crate::commands::bond_mixnode::bond_mixnode;
use crate::commands::claim_delegator_reward::claim_delegator_reward;
use crate::commands::claim_operator_reward::claim_operator_reward;
use crate::commands::compound_delegator_reward::compound_delegator_reward;
use crate::commands::compound_operator_reward::compound_operator_reward;
use crate::commands::create_account::create_account;
use crate::commands::decode_mixnode_key::decode_mixnode_key;
use crate::commands::delegate_to_mixnode::delegate_to_mixnode;
use crate::commands::init_contract::init;
use crate::commands::migrate_contract::migrate;
use crate::commands::send::send;
use crate::commands::unbond_gateway::unbond_gateway;
use crate::commands::unbond_mixnode::unbond_mixnode;
use crate::commands::undelegate_from_mixnode::undelegate_from_mixnode;
use crate::commands::update_profit_percent::update_profit_percent;
use crate::commands::upload_contract::upload;
use crate::commands::vesting_bond_gateway::vesting_bond_gateway;
use crate::commands::vesting_bond_mixnode::vesting_bond_mixnode;
use crate::commands::vesting_claim_delegator_reward::vesting_claim_delegator_reward;
use crate::commands::vesting_claim_operator_reward::vesting_claim_operator_reward;
use crate::commands::vesting_compound_delegator_reward::vesting_compound_delegator_reward;
use crate::commands::vesting_compound_operator_reward::vesting_compound_operator_reward;
use crate::commands::vesting_create_schedule::vesting_create_schedule;
use crate::commands::vesting_delegate_to_mixnode::vesting_delegate_to_mixnode;
use crate::commands::vesting_unbond_gateway::vesting_unbond_gateway;
use crate::commands::vesting_unbond_mixnode::vesting_unbond_mixnode;
use crate::commands::vesting_undelegate_from_mixnode::vesting_undelegate_from_mixnode;
use crate::commands::vesting_update_profit_percent::vesting_update_profit_percent;
use clap::Parser;
use log::{error, warn};
use network_defaults::{
    setup_env,
    var_names::{BECH32_PREFIX, MIX_DENOM},
    NymNetworkDetails,
};
use validator_client::nymd::{self, AccountId, NymdClient, SigningNymdClient};

mod commands;

// we're always going to be using the signing client
pub(crate) type Client = validator_client::nymd::NymdClient<SigningNymdClient>;

#[derive(Debug, Parser)]
pub(crate) enum Command {
    BondGateway(commands::bond_gateway::Args),
    BondMixnode(commands::bond_mixnode::Args),
    ClaimOperatorReward(commands::claim_operator_reward::Args),
    ClaimDelegatorReward(commands::claim_delegator_reward::Args),
    CompoundOperatorReward(commands::compound_operator_reward::Args),
    CompoundDelegatorReward(commands::compound_delegator_reward::Args),
    Init(commands::init_contract::Args),
    Migrate(commands::migrate_contract::Args),
    Upload(commands::upload_contract::Args),
    UnbondMixnode(commands::unbond_mixnode::Args),
    UnbondGateway(commands::unbond_gateway::Args),
    DelegateToMixnode(commands::delegate_to_mixnode::Args),
    UnDelegateFomMixnode(commands::undelegate_from_mixnode::Args),
    Send(commands::send::Args),
    CreateAccount(commands::create_account::Args),
    UpdateProfitPercent(commands::update_profit_percent::Args),
    DecodeMixnodeKey(commands::decode_mixnode_key::Args),
    VestingUpdateProfitPercent(commands::vesting_update_profit_percent::Args),
    VestingBondGateway(commands::vesting_bond_gateway::Args),
    VestingBondMixnode(commands::vesting_bond_mixnode::Args),
    VestingClaimDelegatorRewards(commands::vesting_claim_delegator_reward::Args),
    VestingClaimOperatorRewards(commands::vesting_claim_operator_reward::Args),
    VestingCompoundOperatorRewards(commands::vesting_compound_operator_reward::Args),
    VestingCompoundDelegatorRewards(commands::vesting_compound_delegator_reward::Args),
    VestingCreateSchedule(commands::vesting_create_schedule::Args),
    VestingDelegateToMixnode(commands::vesting_delegate_to_mixnode::Args),
    VestingUnbondGateway(commands::vesting_unbond_gateway::Args),
    VestingUnbondMixnode(commands::vesting_unbond_mixnode::Args),
    VestingUndelegateFromMixnode(commands::vesting_undelegate_from_mixnode::Args),
}

#[derive(Debug, Parser)]
#[clap(name = "validator-client-scripts")]
pub(crate) struct Args {
    #[clap(long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(long)]
    pub(crate) nymd_url: Option<String>,

    #[clap(long)]
    pub(crate) mnemonic: Option<bip39::Mnemonic>,

    // it can only be `None` in the case of contract upload or init
    #[clap(long)]
    pub(crate) mixnet_contract: Option<AccountId>,

    // it can only be `None` in the case of contract upload or init
    #[clap(long)]
    pub(crate) vesting_contract: Option<AccountId>,

    #[clap(subcommand)]
    pub(crate) command: Command,
}

async fn execute(args: Args) {
    let prefix = std::env::var(BECH32_PREFIX).expect("prefix not set");
    let denom = std::env::var(MIX_DENOM).expect("denom not set");
    // doesn't require the client
    if let Command::CreateAccount(args) = args.command {
        return create_account(args, &prefix);
    }

    if let Command::DecodeMixnodeKey(args) = args.command {
        return decode_mixnode_key(args);
    }

    let client = create_client(&args);

    // require the client
    match args.command {
        Command::BondGateway(args) => bond_gateway(client, args, &denom).await,
        Command::BondMixnode(args) => bond_mixnode(client, args, &denom).await,
        Command::ClaimOperatorReward(_args) => claim_operator_reward(client).await,
        Command::ClaimDelegatorReward(args) => claim_delegator_reward(client, args).await,
        Command::CompoundDelegatorReward(args) => compound_delegator_reward(client, args).await,
        Command::CompoundOperatorReward(args) => compound_operator_reward(client, args).await,
        Command::Init(args) => init(client, args, &denom).await,
        Command::Migrate(args) => migrate(client, args).await,
        Command::Upload(args) => upload(client, args).await,
        Command::UnbondMixnode(_) => unbond_mixnode(client).await,
        Command::UnbondGateway(_) => unbond_gateway(client).await,
        Command::Send(args) => send(client, args, &denom).await,
        Command::DelegateToMixnode(args) => delegate_to_mixnode(client, args, &denom).await,
        Command::UnDelegateFomMixnode(args) => undelegate_from_mixnode(client, args).await,
        Command::UpdateProfitPercent(args) => update_profit_percent(client, args).await,
        Command::VestingUpdateProfitPercent(args) => {
            vesting_update_profit_percent(client, args).await
        }
        Command::VestingBondGateway(args) => vesting_bond_gateway(client, args, &denom).await,
        Command::VestingBondMixnode(args) => vesting_bond_mixnode(client, args, &denom).await,
        Command::VestingClaimDelegatorRewards(args) => {
            vesting_claim_delegator_reward(client, args).await
        }
        Command::VestingClaimOperatorRewards(_args) => vesting_claim_operator_reward(client).await,
        Command::VestingCompoundDelegatorRewards(args) => {
            vesting_compound_delegator_reward(client, args).await
        }
        Command::VestingCompoundOperatorRewards(args) => {
            vesting_compound_operator_reward(client, args).await
        }
        Command::VestingCreateSchedule(args) => vesting_create_schedule(client, args, &denom).await,
        Command::VestingDelegateToMixnode(args) => {
            vesting_delegate_to_mixnode(client, args, &denom).await
        }
        Command::VestingUnbondGateway(_args) => vesting_unbond_gateway(client).await,
        Command::VestingUnbondMixnode(_args) => vesting_unbond_mixnode(client).await,
        Command::VestingUndelegateFromMixnode(args) => {
            vesting_undelegate_from_mixnode(client, args).await
        }
        _ => unreachable!(),
    }
}

fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    } else {
        // default to 'Info'
        log_builder.filter(None, log::LevelFilter::Info);
    }

    log_builder
        .filter_module("hyper", log::LevelFilter::Warn)
        .filter_module("tokio_reactor", log::LevelFilter::Warn)
        .filter_module("reqwest", log::LevelFilter::Warn)
        .filter_module("mio", log::LevelFilter::Warn)
        .filter_module("want", log::LevelFilter::Warn)
        .filter_module("sled", log::LevelFilter::Warn)
        .filter_module("tungstenite", log::LevelFilter::Warn)
        .filter_module("tokio_tungstenite", log::LevelFilter::Warn)
        .init();
}

fn create_client(args: &Args) -> Client {
    let network_details = NymNetworkDetails::new_from_env();
    let client_config = nymd::Config::try_from_nym_network_details(&network_details)
        .expect("failed to construct valid validator client config with the provided network");
    NymdClient::connect_with_mnemonic(
        client_config,
        args.nymd_url
            .as_ref()
            .expect("nymd url was not provided")
            .as_str(),
        args.mnemonic
            .as_ref()
            .expect("mnemonic was not provided")
            .clone(),
        None,
    )
    .expect("failed to create the client")
}

async fn wait_for_interrupt() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        error!(
            "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
            e
        );
    }
    println!(
        "Received SIGINT - the gateway will terminate now (threads are not yet nicely stopped, if you see stack traces that's alright)."
    );
}

#[tokio::main]
async fn main() {
    setup_logging();

    let args = Args::parse();
    setup_env(args.config_env_file.clone());

    tokio::select! {
        _ = wait_for_interrupt() => warn!("Received interrupt - the specified command might have not completed!"),
        _ = execute(args) => (),
    }
}
