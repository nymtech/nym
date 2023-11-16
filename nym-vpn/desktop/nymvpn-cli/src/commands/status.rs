use clap::Args;
use console::style;

use crate::cli::RunCommand;

use super::error::CliError;

#[derive(Debug, Args)]
pub struct Status {}

#[async_trait::async_trait]
impl RunCommand for Status {
    async fn run(self) -> Result<(), CliError> {
        let mut client = nymvpn_controller::new_grpc_client()
            .await
            .map_err(|_| CliError::DaemonUnavailable)?;

        let vpn_status = client
            .get_vpn_status(())
            .await
            .map(|res| res.into_inner())
            .map(nymvpn_types::vpn_session::VpnStatus::from)?;

        println!("{}", style(vpn_status).yellow());

        Ok(())
    }
}
