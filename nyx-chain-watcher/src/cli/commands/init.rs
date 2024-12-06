// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::DEFAULT_NYX_CHAIN_WATCHER_ID;
use crate::config::payments_watcher::HttpAuthenticationOptions::AuthorizationBearerToken;
use crate::config::payments_watcher::PaymentWatcherEntry;
use crate::config::{default_config_filepath, Config, ConfigBuilder, PaymentWatcherConfig};
use crate::error::NyxChainWatcherError;
use nym_config::save_unformatted_config_to_file;
use nym_validator_client::nyxd::AccountId;
use std::str::FromStr;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {}

pub(crate) async fn execute(_args: Args) -> Result<(), NyxChainWatcherError> {
    let config_path = default_config_filepath();
    let data_dir = Config::default_data_directory(&config_path)?;

    let builder = ConfigBuilder::new(config_path.clone(), data_dir).with_payment_watcher_config(
        PaymentWatcherConfig {
            watchers: vec![PaymentWatcherEntry {
                id: DEFAULT_NYX_CHAIN_WATCHER_ID.to_string(),
                webhook_url: "https://webhook.site".to_string(),
                watch_for_transfer_recipient_accounts: Some(vec![AccountId::from_str(
                    "n17g9a2pwwkg8m60wf59pq6mv0c2wusg9ukparkz",
                )
                .unwrap()]),
                authentication: Some(AuthorizationBearerToken {
                    token: "1234".to_string(),
                }),
                description: None,
                watch_for_chain_message_types: Some(vec![
                    "/cosmos.bank.v1beta1.MsgSend".to_string(),
                    "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
                ]),
            }],
        },
    );

    let config = builder.build();

    Ok(save_unformatted_config_to_file(&config, &config_path)?)
}
