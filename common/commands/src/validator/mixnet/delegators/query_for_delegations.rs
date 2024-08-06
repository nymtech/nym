// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use log::info;

use crate::context::SigningClientWithNyxd;
use crate::utils::{pretty_cosmwasm_coin, show_error_passthrough};

use comfy_table::Table;
use cosmwasm_std::Addr;
use nym_mixnet_contract_common::{Delegation, PendingEpochEvent, PendingEpochEventKind};
use nym_validator_client::nyxd::contract_traits::PagedMixnetQueryClient;

#[derive(Debug, Parser)]
pub struct Args {}

pub async fn execute(_args: Args, client: SigningClientWithNyxd) {
    info!(
        "Getting delegations for account {}...",
        client.nyxd.address()
    );

    let delegations = client
        .nyxd
        .get_all_delegator_delegations(&client.nyxd.address())
        .await
        .map_err(show_error_passthrough);

    let mixnet_contract_events = client
        .nyxd
        .get_all_pending_epoch_events()
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
}

async fn to_iso_timestamp(block_height: u32, client: &SigningClientWithNyxd) -> String {
    match client.nyxd.get_block_timestamp(Some(block_height)).await {
        Ok(res) => res.to_rfc3339(),
        Err(_e) => "-".to_string(),
    }
}

async fn print_delegations(delegations: Vec<Delegation>, client: &SigningClientWithNyxd) {
    let mut table = Table::new();

    table.set_header(vec!["Timestamp", "Mix Id", "Delegation", "Proxy"]);

    for delegation in delegations {
        table.add_row(vec![
            to_iso_timestamp(delegation.height as u32, client).await,
            delegation.mix_id.to_string(),
            pretty_cosmwasm_coin(&delegation.amount),
            delegation
                .proxy
                .map(Addr::into_string)
                .unwrap_or_else(|| "-".into()),
        ]);
    }

    println!("{table}");
}

async fn print_delegation_events(events: Vec<PendingEpochEvent>, client: &SigningClientWithNyxd) {
    let mut table = Table::new();

    table.set_header(vec![
        "Timestamp",
        "Mix id",
        "Delegation",
        "Event Type",
        "Proxy",
    ]);

    for event in events {
        match event.event.kind {
            PendingEpochEventKind::Delegate {
                owner,
                mix_id,
                amount,
                proxy,
                ..
            } => {
                if owner.as_str() == client.nyxd.address().as_ref() {
                    table.add_row(vec![
                        "not-sure-if-applicable".into(),
                        mix_id.to_string(),
                        pretty_cosmwasm_coin(&amount),
                        "Delegate".to_string(),
                        proxy.map(Addr::into_string).unwrap_or_else(|| "-".into()),
                    ]);
                }
            }
            PendingEpochEventKind::Undelegate {
                owner,
                mix_id,
                proxy,
                ..
            } => {
                if owner.as_str() == client.nyxd.address().as_ref() {
                    table.add_row(vec![
                        "not-sure-if-applicable".into(),
                        mix_id.to_string(),
                        "-".to_string(),
                        "Undelegate".to_string(),
                        proxy.map(Addr::into_string).unwrap_or_else(|| "-".into()),
                    ]);
                }
            }
            _ => {}
        }
    }

    println!("{table}");
}
