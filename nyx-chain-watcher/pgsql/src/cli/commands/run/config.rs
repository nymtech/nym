use crate::cli::commands::run::args::Args;
use crate::cli::DEFAULT_NYX_CHAIN_WATCHER_ID;
use crate::config::payments_watcher::{HttpAuthenticationOptions, PaymentWatcherConfig};
use crate::config::{default_config_filepath, Config, ConfigBuilder, PaymentWatchersConfig};
use crate::error::NyxChainWatcherError;
use tracing::{info, warn};

pub(crate) fn get_run_config(args: Args) -> Result<Config, NyxChainWatcherError> {
    info!("{args:#?}");

    let Args {
        ref watch_for_transfer_recipient_accounts,
        mut watch_for_chain_message_types,
        webhook_auth,
        ref chain_watcher_db_path,
        webhook_url,
        ..
    } = args;

    // if there are no args set, then try load the config
    if args.watch_for_transfer_recipient_accounts.is_empty()
        && args.watch_for_transfer_recipient_accounts.is_empty()
        && args.chain_watcher_db_path.is_none()
    {
        info!("Loading default config file...");
        return Config::read_from_toml_file_in_default_location();
    }

    // set default messages
    if watch_for_chain_message_types.is_empty() {
        watch_for_chain_message_types = vec!["/cosmos.bank.v1beta1.MsgSend".to_string()];
    }

    // warn if no accounts set
    if watch_for_transfer_recipient_accounts.is_empty() {
        warn!(
            "You did not specify any accounts to watch in {}. Only chain data will be stored.",
            crate::env::vars::NYX_CHAIN_WATCHER_WATCH_ACCOUNTS
        );
    }

    let config_path = default_config_filepath();
    let data_dir = Config::default_data_directory(&config_path)?;

    let mut builder = ConfigBuilder::new(
        config_path,
        data_dir,
        args.chain_history_db_connection_string,
    );

    if let Some(db_path) = chain_watcher_db_path {
        info!("Overriding database url with '{db_path}'");
        builder = builder.with_db_path(db_path.clone());
    }

    if let Some(webhook_url) = webhook_url {
        let authentication =
            webhook_auth.map(|token| HttpAuthenticationOptions::AuthorizationBearerToken { token });

        let watcher_config = PaymentWatchersConfig {
            watchers: vec![PaymentWatcherConfig {
                id: DEFAULT_NYX_CHAIN_WATCHER_ID.to_string(),
                description: None,
                watch_for_transfer_recipient_accounts: watch_for_transfer_recipient_accounts
                    .clone(),
                watch_for_chain_message_types,
                webhook_url,
                authentication,
            }],
        };

        info!("Overriding watcher config with env vars");

        builder = builder.with_payment_watcher_config(watcher_config);
    } else {
        warn!(
            "You did not specify a webhook in {}. Only database items will be stored.",
            crate::env::vars::NYX_CHAIN_WATCHER_WEBHOOK_URL
        );
    }

    Ok(builder.build())
}
