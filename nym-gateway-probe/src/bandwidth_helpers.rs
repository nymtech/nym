// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, bail};
use nym_bandwidth_controller::error::BandwidthControllerError;
use nym_client_core::client::base_client::storage::OnDiskPersistent;
use nym_credentials_interface::TicketType;
use nym_node_status_client::models::AttachedTicketMaterials;
use nym_sdk::bandwidth::BandwidthImporter;
use nym_sdk::mixnet::{DisconnectedMixnetClient, EphemeralCredentialStorage};
use nym_validator_client::nyxd::error::NyxdError;
use std::time::Duration;
use tracing::{error, info};

pub(crate) async fn import_bandwidth(
    bandwidth_importer: BandwidthImporter<'_, EphemeralCredentialStorage>,
    attached_ticket_materials: AttachedTicketMaterials,
) -> anyhow::Result<()> {
    // 1. import all auxiliary data
    for master_vk in attached_ticket_materials.master_verification_keys {
        let key = master_vk.try_unpack()?;
        info!(
            "importing master verification key for epoch {}",
            key.epoch_id
        );
        bandwidth_importer
            .import_master_verification_key(&key)
            .await?;
    }
    for coin_index_signatures in attached_ticket_materials.coin_indices_signatures {
        let sigs = coin_index_signatures.try_unpack()?;
        info!("importing coin index signatures epoch {}", sigs.epoch_id);
        bandwidth_importer
            .import_coin_index_signatures(&sigs)
            .await?;
    }
    for expiration_date_signatures in attached_ticket_materials.expiration_date_signatures {
        let sigs = expiration_date_signatures.try_unpack()?;
        info!(
            "importing expiration date signatures epoch {} and expiration {}",
            sigs.epoch_id, sigs.expiration_date
        );
        bandwidth_importer
            .import_expiration_date_signatures(&sigs)
            .await?;
    }

    // 2. import actual tickets
    for ticket in attached_ticket_materials.attached_tickets {
        let ticketbook = ticket.ticketbook.try_unpack()?;
        info!(
            "importing partial ticketbook {}. index to use: {}",
            ticketbook.ticketbook_type(),
            ticket.usable_index
        );
        bandwidth_importer
            .import_partial_ticketbook(&ticketbook, ticket.usable_index, ticket.usable_index)
            .await?;
    }

    Ok(())
}

pub(crate) async fn acquire_bandwidth(
    mnemonic: &str,
    disconnected_mixnet_client: &DisconnectedMixnetClient<OnDiskPersistent>,
    ticketbook_type: TicketType,
) -> anyhow::Result<()> {
    // TODO: make it configurable
    const MAX_RETRIES: usize = 50;
    for i in 0..MAX_RETRIES {
        let attempt = i + 1; // since humans usually don't count from 0 in this instance
        info!(
            "attempt {attempt}/{MAX_RETRIES} for attempting to acquire {ticketbook_type} bandwidth"
        );
        let bw_client = disconnected_mixnet_client
            .create_bandwidth_client(mnemonic.to_string(), ticketbook_type)
            .await?;
        info!("Calling bandwidth controller acquire() for {ticketbook_type}");
        match bw_client.acquire().await {
            Ok(_) => {
                if i > 0 {
                    info!(
                        "managed to acquire {ticketbook_type} bandwidth after {attempt} attempts",
                    );
                }
                return Ok(());
            }
            Err(nym_sdk::Error::CredentialIssuanceError { source }) => match source {
                nym_credential_utils::errors::Error::BandwidthControllerError(
                    BandwidthControllerError::Nyxd(nyxd_error),
                ) => match nyxd_error {
                    // happens when sequence issue occurs during tx delivery
                    NyxdError::BroadcastTxErrorDeliverTx {
                        hash,
                        height,
                        code,
                        raw_log,
                    } => {
                        // unfortunately at this point we have to do string matching as the log
                        // is returned from the go nyxd binary
                        if raw_log.contains("account sequence mismatch") {
                            error!(
                                "another process is using the same mnemonic. we failed to broadcast transaction {hash} due to mismatched sequence number"
                            )
                        } else {
                            return Err(NyxdError::BroadcastTxErrorDeliverTx {
                                hash,
                                height,
                                code,
                                raw_log,
                            }
                            .into());
                        }
                    }
                    // happens when sequence issue occurs during tx simulate
                    NyxdError::AbciError {
                        code,
                        log,
                        pretty_log,
                    } => {
                        // unfortunately at this point we have to do string matching as the log
                        // is returned from the go nyxd binary
                        if log.contains("account sequence mismatch") {
                            error!(
                                "another process is using the same mnemonic. we failed to simulate transaction due to mismatched sequence number"
                            )
                        } else {
                            return Err(NyxdError::AbciError {
                                code,
                                log,
                                pretty_log,
                            }
                            .into());
                        }
                    }
                    other => {
                        return Err(other)
                            .context("another nyxd failure during bandwidth acquisition");
                    }
                },
                other => {
                    return Err(other.into());
                }
            },
            Err(other) => {
                return Err(other.into());
            }
        }

        // add a bit of backoff as if the rpc node is slightly out of sync,
        // we might use our retry budget for abci queries to the simulate endpoint
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    bail!("failed to acquire bandwidth after {MAX_RETRIES} attempts")
}
