use async_trait::async_trait;
use clap::Args;
use console::style;

use crate::cli::RunCommand;

use super::error::CliError;

#[derive(Args, Debug)]
pub struct SignOut;

#[async_trait]
impl RunCommand for SignOut {
    async fn run(self) -> Result<(), CliError> {
        let mut client = nymvpn_controller::new_grpc_client()
            .await
            .map_err(|_| CliError::DaemonUnavailable)?;

        client.account_sign_out(()).await?;

        println!("{}", style("Successfully signed out").yellow());

        Ok(())
    }
}
