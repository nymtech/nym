use clap::Parser;
use log::{error, info};
use nym_name_service_common::NameId;
use nym_validator_client::nyxd::{contract_traits::NameServiceSigningClient, error::NyxdError};
use tap::TapFallible;

use crate::context::SigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub id: NameId,
}

pub async fn delete(args: Args, client: SigningClient) -> Result<(), NyxdError> {
    info!("Deleting registered name alias with id {}", args.id);

    let res = client
        .delete_name_by_id(args.id, None)
        .await
        .tap_err(|err| error!("Failed to delete name: {err:#?}"))?;

    info!("Deleted: {res:?}");
    Ok(())
}
