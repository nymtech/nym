use crate::cli::commands::run::args::Args;
use crate::cli::DEFAULT_NYM_DATA_OBSERVATORY_ID;
use crate::config::data_observatory::{HttpAuthenticationOptions, WebhookConfig};
use crate::config::{default_config_filepath, Config, ConfigBuilder, DataObservatoryConfig};
use crate::error::NymDataObservatoryError;
use tracing::{info, warn};

pub(crate) fn get_run_config(args: Args) -> Result<Config, NymDataObservatoryError> {
    info!("{args:#?}");

    let Args {
        mut watch_for_chain_message_types,
        webhook_auth,
        webhook_url,
        ..
    } = args;

    // if there are no args set, then try load the config
    if args.db_connection_string.is_none() {
        info!("Loading default config file...");
        return Config::read_from_toml_file_in_default_location();
    }

    // set default messages
    if watch_for_chain_message_types.is_empty() {
        watch_for_chain_message_types = vec!["/cosmos.bank.v1beta1.MsgSend".to_string()];
    }

    let config_path = default_config_filepath();
    let data_dir = Config::default_data_directory(&config_path)?;

    if args.db_connection_string.is_none() {
        return Err(NymDataObservatoryError::DbConnectionStringMissing);
    }

    let mut builder = ConfigBuilder::new(
        config_path,
        data_dir,
        args.db_connection_string
            .expect("db connection string is required"),
    );

    if let Some(webhook_url) = webhook_url {
        let authentication =
            webhook_auth.map(|token| HttpAuthenticationOptions::AuthorizationBearerToken { token });

        let watcher_config = DataObservatoryConfig {
            webhooks: vec![WebhookConfig {
                id: DEFAULT_NYM_DATA_OBSERVATORY_ID.to_string(),
                description: None,
                watch_for_chain_message_types,
                webhook_url,
                authentication,
            }],
        };

        info!("Overriding watcher config with env vars");

        builder = builder.with_data_observatory_config(watcher_config);
    } else {
        warn!(
            "You did not specify a webhook in {}. Only database items will be stored.",
            crate::env::vars::NYM_DATA_OBSERVATORY_WEBHOOK_URL
        );
    }

    Ok(builder.build())
}
