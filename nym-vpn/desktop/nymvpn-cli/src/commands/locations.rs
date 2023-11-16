use async_trait::async_trait;
use clap::Args;
use console::style;
use validator::Validate;

use crate::cli::RunCommand;

use super::error::CliError;

#[derive(Args, Debug, Validate)]
pub struct ListLocations {}

pub async fn list_locations() -> Result<Vec<nymvpn_types::location::Location>, CliError> {
    let mut client = nymvpn_controller::new_grpc_client()
        .await
        .map_err(|_| CliError::DaemonUnavailable)?;

    let locations = client.get_locations(()).await?;

    Ok(locations.into_inner().into())
}

#[async_trait]
impl RunCommand for ListLocations {
    async fn run(self) -> Result<(), CliError> {
        let locations = list_locations().await?;

        for location in locations {
            if location.state.is_some() {
                println!(
                    "{}, {}, {}",
                    style(location.city).white(),
                    style(location.state.unwrap()).white().dim(),
                    style(location.country).white().dim()
                );
            } else {
                println!(
                    "{}, {}",
                    style(location.city).white(),
                    style(location.country).white().dim()
                );
            }
        }

        Ok(())
    }
}
