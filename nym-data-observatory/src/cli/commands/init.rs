// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::DEFAULT_NYM_DATA_OBSERVATORY_ID;
use crate::config::data_observatory::HttpAuthenticationOptions::AuthorizationBearerToken;
use crate::config::data_observatory::WebhookConfig;
use crate::config::{default_config_filepath, Config, ConfigBuilder, DataObservatoryConfig};
use crate::env::vars::*;
use crate::error::NymDataObservatoryError;
use nym_config::save_unformatted_config_to_file;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// (Override) Postgres connection string for data storage
    #[arg(long, env = NYM_DATA_OBSERVATORY_DB_URL, alias = "db_url")]
    pub(crate) chain_history_db_connection_string: String,
}

pub(crate) async fn execute(args: Args) -> Result<(), NymDataObservatoryError> {
    let config_path = default_config_filepath();
    let data_dir = Config::default_data_directory(&config_path)?;

    let builder = ConfigBuilder::new(
        config_path.clone(),
        data_dir,
        args.chain_history_db_connection_string,
    )
    .with_data_observatory_config(DataObservatoryConfig {
        webhooks: vec![WebhookConfig {
            id: DEFAULT_NYM_DATA_OBSERVATORY_ID.to_string(),
            webhook_url: "https://webhook.site".to_string(),
            authentication: Some(AuthorizationBearerToken {
                token: "1234".to_string(),
            }),
            description: None,
            watch_for_chain_message_types: vec![
                "/cosmos.bank.v1beta1.MsgSend".to_string(),
                "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
            ],
        }],
    });

    let config = builder.build();

    Ok(save_unformatted_config_to_file(&config, &config_path)?)
}
