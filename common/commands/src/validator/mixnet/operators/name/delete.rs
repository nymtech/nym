use clap::Parser;
use log::info;
use nym_name_service_common::NameId;
use nym_validator_client::nyxd::traits::NameServiceSigningClient;

use crate::context::SigningClient;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub id: NameId,
}

pub async fn delete(args: Args, client: SigningClient) {
    info!("Deleting registered name alias with id {}", args.id);

    let res = client
        .delete_name_by_id(args.id, None)
        .await
        .expect("Failed to delete name");

    info!("Deleted: {res:?}");
}
