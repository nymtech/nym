// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::env::vars::*;
use nym_validator_client::nyxd::AccountId;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// (Override) SQLite database file path for chain watcher
    #[arg(long, env = NYX_CHAIN_WATCHER_DATABASE_PATH)]
    pub(crate) chain_watcher_db_path: Option<String>,

    /// (Override) SQLite database file path for chain scraper history
    #[arg(long, env = NYX_CHAIN_WATCHER_HISTORY_DATABASE_PATH)]
    pub(crate) chain_history_db_path: Option<String>,

    /// (Override) Watch for transfers to these recipient accounts
    #[clap(
        long,
        value_delimiter = ',',
        env = NYX_CHAIN_WATCHER_WATCH_ACCOUNTS
    )]
    pub watch_for_transfer_recipient_accounts: Option<Vec<AccountId>>,

    /// (Override) Watch for chain messages of these types
    #[clap(
        long,
        value_delimiter = ',',
        env = NYX_CHAIN_WATCHER_WATCH_CHAIN_MESSAGE_TYPES
    )]
    pub watch_for_chain_message_types: Option<Vec<String>>,

    /// (Override) The webhook to call when we find something
    #[clap(
        long,
        env = NYX_CHAIN_WATCHER_WEBHOOK_URL
    )]
    pub webhook_url: Option<String>,

    /// (Override) Optionally, authenticate with the webhook
    #[clap(
        long,
        env = NYX_CHAIN_WATCHER_WEBHOOK_AUTH
    )]
    pub webhook_auth: Option<String>,
}

/*impl Args {
    pub(super) fn take_mnemonic(&mut self) -> Option<Zeroizing<bip39::Mnemonic>> {
        self.entry_gateway.mnemonic.take().map(Zeroizing::new)
    }
}

impl Args {
    pub(crate) fn build_config(self) -> Result<Config, NymNodeError> {
        let config_path = self.config.config_path();
        let data_dir = Config::default_data_directory(&config_path)?;

        let id = self
            .config
            .id()
            .clone()
            .ok_or(NymNodeError::MissingInitArg {
                section: "global".to_string(),
                name: "id".to_string(),
            })?;

        let config = ConfigBuilder::new(id, config_path.clone(), data_dir.clone())
            .with_mode(self.mode.unwrap_or_default())
            .with_host(self.host.build_config_section())
            .with_http(self.http.build_config_section())
            .with_mixnet(self.mixnet.build_config_section())
            .with_wireguard(self.wireguard.build_config_section(&data_dir))
            .with_storage_paths(NymNodePaths::new(&data_dir))
            .with_mixnode(self.mixnode.build_config_section())
            .with_entry_gateway(self.entry_gateway.build_config_section(&data_dir))
            .with_exit_gateway(self.exit_gateway.build_config_section(&data_dir))
            .build();

        Ok(config)
    }

    pub(crate) fn override_config(self, mut config: Config) -> Config {
        if let Some(mode) = self.mode {
            config.mode = mode;
        }
        config.host = self.host.override_config_section(config.host);
        config.http = self.http.override_config_section(config.http);
        config.mixnet = self.mixnet.override_config_section(config.mixnet);
        config.wireguard = self.wireguard.override_config_section(config.wireguard);
        config.mixnode = self.mixnode.override_config_section(config.mixnode);
        config.entry_gateway = self
            .entry_gateway
            .override_config_section(config.entry_gateway);
        config.exit_gateway = self
            .exit_gateway
            .override_config_section(config.exit_gateway);
        config
    }
}
*/
