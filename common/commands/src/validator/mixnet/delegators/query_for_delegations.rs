// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::info;

use crate::context::SigningClientWithValidatorAPI;
use crate::utils::{pretty_cosmwasm_coin, show_error_passthrough};

use comfy_table::Table;
use mixnet_contract_common::mixnode::DelegationEvent;
use mixnet_contract_common::Delegation;

#[derive(Debug, Parser)]
pub struct Args {}

pub async fn execute(_args: Args, client: SigningClientWithValidatorAPI) {
    info!(
        "Getting delegations for account {}...",
        client.nymd.address()
    );

    let delegations = client
        .get_all_delegator_delegations(client.nymd.address())
        .await
        .map_err(show_error_passthrough);

    let mixnet_contract_events = client
        .nymd
        .get_pending_delegation_events(client.nymd.address().to_string(), None)
        .await
        .map_err(show_error_passthrough);

    let vesting_contract = client.nymd.vesting_contract_address();

    let vesting_contract_events = client
        .nymd
        .get_pending_delegation_events(
            client.nymd.address().to_string(),
            Some(vesting_contract.to_string()),
        )
        .await
        .map_err(show_error_passthrough);

    if let Ok(res) = delegations {
        println!();
        if res.is_empty() {
            println!("This account has not delegated any tokens to mixnodes");
        } else {
            println!("Delegations:");
            print_delegations(res, &client).await;
        }
    }
    if let Ok(res) = mixnet_contract_events {
        if !res.is_empty() {
            println!();
            println!("Pending delegations (liquid tokens):");
            print_delegation_events(res, &client).await;
        }
    }
    if let Ok(res) = vesting_contract_events {
        if !res.is_empty() {
            println!();
            println!("Pending delegations (locked tokens):");
            print_delegation_events(res, &client).await;
        }
    }
}

async fn to_iso_timestamp(block_height: u32, client: &SigningClientWithValidatorAPI) -> String {
    match client.nymd.get_block_timestamp(Some(block_height)).await {
        Ok(res) => res.to_rfc3339(),
        Err(_e) => "-".to_string(),
    }
}

async fn print_delegations(delegations: Vec<Delegation>, client: &SigningClientWithValidatorAPI) {
    let mut table = Table::new();

    table.set_header(vec!["Timestamp", "Identity Key", "Delegation", "Proxy"]);

    for delegation in delegations {
        table.add_row(vec![
            to_iso_timestamp(delegation.block_height as u32, client).await,
            delegation.node_identity.to_string(),
            pretty_cosmwasm_coin(&delegation.amount),
            format!("{:?}", delegation.proxy),
        ]);
    }

    println!("{table}");
}

async fn print_delegation_events(
    events: Vec<DelegationEvent>,
    client: &SigningClientWithValidatorAPI,
) {
    let mut table = Table::new();

    table.set_header(vec![
        "Timestamp",
        "Identity Key",
        "Delegation",
        "Event Type",
    ]);

    for event in events {
        match event {
            DelegationEvent::Delegate(delegation) => {
                table.add_row(vec![
                    to_iso_timestamp(delegation.block_height as u32, client).await,
                    delegation.node_identity.to_string(),
                    pretty_cosmwasm_coin(&delegation.amount),
                    "Delegate".to_string(),
                ]);
            }
            DelegationEvent::Undelegate(undelegate) => {
                table.add_row(vec![
                    to_iso_timestamp(undelegate.block_height() as u32, client).await,
                    undelegate.mix_identity().to_string(),
                    "-".to_string(),
                    "Undelegate".to_string(),
                ]);
            }
        }
    }

    println!("{table}");
}
