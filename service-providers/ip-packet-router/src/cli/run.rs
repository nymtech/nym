use crate::cli::try_load_current_config;
use crate::cli::{override_config, OverrideConfig};
use clap::Args;
use nym_client_core::cli_helpers::client_run::CommonClientRunArgs;
use nym_ip_packet_router::error::IpPacketRouterError;

#[allow(clippy::struct_excessive_bools)]
#[derive(Args, Clone)]
pub(crate) struct Run {
    #[command(flatten)]
    common_args: CommonClientRunArgs,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            nym_apis: None,
            nyxd_urls: run_config.common_args.nyxd_urls,
            enabled_credentials_mode: run_config.common_args.enabled_credentials_mode,
        }
    }
}

pub(crate) async fn execute(args: &Run) -> Result<(), IpPacketRouterError> {
    let mut config = try_load_current_config(&args.common_args.id).await?;
    config = override_config(config, OverrideConfig::from(args.clone()));
    log::debug!("Using config: {config:#?}");

    log::info!("Starting ip packet router service provider");
    let mut server = nym_ip_packet_router::IpPacketRouter::new(config);
    if let Some(custom_mixnet) = &args.common_args.custom_mixnet {
        server = server.with_stored_topology(custom_mixnet)?
    }

    server.run_service_provider().await
}
