use crate::errors::{Error, Result};
use log::*;
use nym_bandwidth_controller::acquire::{
    get_ticket_book, query_and_persist_required_global_signatures,
};
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_config::DEFAULT_DATA_DIR;
use nym_credential_storage::persistent_storage::PersistentStorage;
use nym_credential_storage::storage::Storage;
use nym_ecash_time::ecash_default_expiration_date;
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::nyxd::contract_traits::{
    dkg_query_client::EpochState, DkgQueryClient, EcashSigningClient,
};
use std::path::PathBuf;
use std::time::Duration;
use time::OffsetDateTime;

pub async fn issue_credential<C, S>(client: &C, storage: &S, client_id: &[u8]) -> Result<()>
where
    C: DkgQueryClient + EcashSigningClient + Send + Sync,
    S: Storage,
    <S as Storage>::StorageError: Send + Sync + 'static,
{
    block_until_ecash_is_available(client).await?;
    info!("Starting to deposit funds, don't kill the process");

    if let Ok(recovered_ticketbooks) = recover_deposits(client, storage).await {
        if recovered_ticketbooks != 0 {
            info!("managed to recover {recovered_ticketbooks} ticket books. no need to make fresh deposit");
            return Ok(());
        }
    };

    let epoch_id = client.get_current_epoch().await?.epoch_id;
    let apis = all_ecash_api_clients(client, epoch_id).await?;
    let ticketbook_expiration = ecash_default_expiration_date();

    // make sure we have all required coin indices and expiration date signatures before attempting the deposit
    query_and_persist_required_global_signatures(
        storage,
        epoch_id,
        ticketbook_expiration,
        apis.clone(),
    )
    .await?;

    let issuance_data = nym_bandwidth_controller::acquire::make_deposit(
        client,
        client_id,
        Some(ticketbook_expiration),
    )
    .await?;
    info!("Deposit done");

    if get_ticket_book(&issuance_data, client, storage, Some(apis))
        .await
        .is_err()
    {
        error!("failed to obtain credential. saving recovery data...");

        storage.insert_pending_ticketbook(&issuance_data).await.inspect_err(|err| {
            let deposit = issuance_data.deposit_id();
            error!("could not save the recovery data for deposit {deposit}: {err}. the data will unfortunately get lost")
        }).map_err(Error::storage_error)?
    }

    info!("Succeeded adding a ticketbook");

    Ok(())
}

pub async fn setup_persistent_storage(client_home_directory: PathBuf) -> PersistentStorage {
    let data_dir = client_home_directory.join(DEFAULT_DATA_DIR);
    let paths = CommonClientPaths::new_base(data_dir);
    let db_path = paths.credentials_database;

    nym_credential_storage::initialise_persistent_storage(db_path).await
}

pub async fn block_until_ecash_is_available<C>(client: &C) -> Result<()>
where
    C: DkgQueryClient + Send + Sync,
{
    loop {
        let epoch = client.get_current_epoch().await?;
        let current_timestamp_secs = OffsetDateTime::now_utc().unix_timestamp() as u64;

        if epoch.state.is_final() {
            break;
        } else if let Some(final_timestamp) = epoch.final_timestamp_secs() {
            // Use 1 additional second to not start the next iteration immediately and spam get_current_epoch queries
            let secs_until_final = final_timestamp.saturating_sub(current_timestamp_secs) + 1;
            info!("Approximately {} seconds until coconut is available. Sleeping until then. You can safely kill the process at any moment.", secs_until_final);
            tokio::time::sleep(Duration::from_secs(secs_until_final)).await;
        } else if matches!(epoch.state, EpochState::WaitingInitialisation) {
            info!("dkg hasn't been initialised yet and it is not known when it will be. Going to check again later");
            tokio::time::sleep(Duration::from_secs(60 * 5)).await;
        } else {
            // this should never be the case since the only case where final timestamp is unknown is when it's waiting for initialisation,
            // but let's guard ourselves against future changes
            info!("it is unknown when ecash will be come available. Going to check again later");
            tokio::time::sleep(Duration::from_secs(60 * 5)).await;
        }
    }

    Ok(())
}

pub async fn recover_deposits<C, S>(client: &C, storage: &S) -> Result<usize>
where
    C: DkgQueryClient + Send + Sync,
    S: Storage,
    <S as Storage>::StorageError: Send + Sync + 'static,
{
    info!("checking for any incomplete previous issuance attempts...");

    let incomplete = storage
        .get_pending_ticketbooks()
        .await
        .map_err(Error::storage_error)?;
    info!(
        "we recovered {} incomplete ticketbook issuances",
        incomplete.len()
    );

    let mut recovered_books = 0;
    for issuance in incomplete {
        let deposit = issuance.pending_ticketbook.deposit_id();
        if issuance.pending_ticketbook.expired() {
            warn!("ticketbook data associated with deposit {deposit} has expired. if you haven't contacted more than 1/3 of signers. it could still be recoverable (but out of scope of this library)");
            continue;
        }

        if issuance.pending_ticketbook.check_expiration_date() {
            warn!("deposit {deposit} was made with a different expiration date, it's validity will be shorter than the max one");
        }

        match get_ticket_book(&issuance.pending_ticketbook, client, storage, None).await {
            Err(err) => error!("could not recover deposit {deposit} due to: {err}"),
            Ok(_) => {
                info!("managed to recover deposit {deposit}! the ticketbook has been added to the storage");
                storage
                    .remove_pending_ticketbook(issuance.pending_id)
                    .await
                    .map_err(Error::storage_error)?;
                recovered_books += 1;
            }
        }
    }

    Ok(recovered_books)
}
