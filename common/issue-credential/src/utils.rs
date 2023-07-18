use log::*;
use std::process::exit;
use std::time::{Duration, SystemTime};

use nym_bandwidth_controller::acquire::state::State;
use nym_credential_storage::persistent_storage::PersistentStorage;
use nym_validator_client::nyxd::traits::DkgQueryClient;
use nym_validator_client::nyxd::DirectSigningNyxdClient;
use nym_validator_client::Client;

use crate::errors::Result;
use crate::recovery_storage::RecoveryStorage;

const SAFETY_BUFFER_SECS: u64 = 60; // 1 minute

pub async fn block_until_coconut_is_available(
    client: &Client<DirectSigningNyxdClient>,
) -> Result<()> {
    loop {
        let epoch = client.nyxd.get_current_epoch().await?;
        println!("{:?}", epoch);
        let current_timestamp_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        if epoch.state.is_final() {
            println!(
                "{}, {}",
                current_timestamp_secs,
                epoch.finish_timestamp.seconds()
            );
            if current_timestamp_secs + SAFETY_BUFFER_SECS >= epoch.finish_timestamp.seconds() {
                info!("In the next {} minute(s), a transition will take place in the coconut system. Deposits should be halted in this time for safety reasons.", SAFETY_BUFFER_SECS / 60);
                exit(0);
            }

            break;
        } else {
            // Use 1 additional second to not start the next iteration immediately and spam get_current_epoch queries
            let secs_until_final = epoch
                .final_timestamp_secs()
                .saturating_sub(current_timestamp_secs)
                + 1;
            info!("Approximately {} seconds until coconut is available. Sleeping until then. You can safely kill the process at any moment.", secs_until_final);
            std::thread::sleep(Duration::from_secs(secs_until_final));
        }
    }

    Ok(())
}

pub async fn recover_credentials<C: DkgQueryClient + Send + Sync>(
    client: &C,
    recovery_storage: &RecoveryStorage,
    shared_storage: &PersistentStorage,
) -> Result<()> {
    for voucher in recovery_storage.unconsumed_vouchers()? {
        let state = State::new(voucher);
        if let Err(e) =
            nym_bandwidth_controller::acquire::get_credential(&state, client, shared_storage).await
        {
            error!(
                "Could not recover deposit {} due to {:?}, try again later",
                state.voucher.tx_hash(),
                e
            )
        } else {
            info!(
                "Converted deposit {} to a credential, removing recovery data for it",
                state.voucher.tx_hash()
            );
            if let Err(e) = recovery_storage.remove_voucher(state.voucher.tx_hash().to_string()) {
                warn!("Could not remove recovery data - {:?}", e);
            }
        }
    }

    Ok(())
}
