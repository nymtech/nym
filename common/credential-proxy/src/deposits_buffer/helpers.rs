// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::CredentialProxyError;
use crate::shared_state::nyxd_client::ChainClient;
use crate::storage::models::StorableEcashDeposit;
use nym_compact_ecash::WithdrawalRequest;
use nym_credentials::IssuanceTicketBook;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::cosmwasm_client::ContractResponseData;
use nym_validator_client::nyxd::{Coin, Hash};
use rand::rngs::OsRng;
use std::fmt::Debug;
use time::OffsetDateTime;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument};
use zeroize::Zeroizing;

pub struct BufferedDeposit {
    pub deposit_id: u32,

    // note: this type implements `ZeroizeOnDrop`
    pub ed25519_private_key: ed25519::PrivateKey,
}

impl TryFrom<StorableEcashDeposit> for BufferedDeposit {
    type Error = CredentialProxyError;

    fn try_from(deposit: StorableEcashDeposit) -> Result<Self, Self::Error> {
        let ed25519_private_key = ed25519::PrivateKey::from_bytes(
            deposit.ed25519_deposit_private_key.as_ref(),
        )
        .map_err(|err| CredentialProxyError::DatabaseInconsistency {
            reason: format!("one of the stored deposit ed25519 private keys is malformed: {err}"),
        })?;

        Ok(BufferedDeposit {
            deposit_id: deposit.deposit_id,
            ed25519_private_key,
        })
    }
}

impl BufferedDeposit {
    pub fn new(deposit_id: u32, ed25519_private_key: ed25519::PrivateKey) -> Self {
        BufferedDeposit {
            deposit_id,
            ed25519_private_key,
        }
    }

    pub fn sign_ticketbook_plaintext(
        &self,
        withdrawal_request: &WithdrawalRequest,
    ) -> ed25519::Signature {
        let plaintext = IssuanceTicketBook::request_plaintext(withdrawal_request, self.deposit_id);
        self.ed25519_private_key.sign(plaintext)
    }
}

pub struct PerformedDeposits {
    pub deposits_data: Vec<BufferedDeposit>,

    // shared by all performed deposits as they were included in the same tx
    pub tx_hash: Hash,
    pub requested_on: OffsetDateTime,
    pub deposit_amount: Coin,
}

impl PerformedDeposits {
    pub(crate) fn to_storable(&self) -> Vec<StorableEcashDeposit> {
        self.deposits_data
            .iter()
            .map(|d| StorableEcashDeposit {
                deposit_id: d.deposit_id,
                deposit_tx_hash: self.tx_hash.to_string(),
                requested_on: self.requested_on,
                deposit_amount: self.deposit_amount.to_string(),
                ed25519_deposit_private_key: Zeroizing::new(d.ed25519_private_key.to_bytes()),
            })
            .collect()
    }
}

#[instrument(skip(client, cancellation_on_critical_failure), err(Display))]
pub async fn make_deposits_request(
    client: &ChainClient,
    deposit_amount: Coin,
    memo: impl Into<String> + Debug,
    amount: usize,
    cancellation_on_critical_failure: &CancellationToken,
) -> Result<PerformedDeposits, CredentialProxyError> {
    let requested_on = OffsetDateTime::now_utc();
    let chain_write_permit = client.start_chain_tx().await;
    let mut rng = OsRng;

    let keys = (0..amount)
        .map(|_| ed25519::PrivateKey::new(&mut rng))
        .collect::<Vec<_>>();

    info!("starting {amount} deposits");
    let mut contents = Vec::new();
    for key in &keys {
        let public_key: ed25519::PublicKey = key.into();
        contents.push((public_key.to_base58_string(), deposit_amount.clone()));
    }

    let execute_res = chain_write_permit
        .make_deposits(memo.into(), contents)
        .await?;

    let tx_hash = execute_res.transaction_hash;
    info!("{amount} deposits made in transaction: {tx_hash}");

    let contract_data = match execute_res.to_contract_data() {
        Ok(contract_data) => contract_data,
        Err(err) => {
            // that one is tricky. deposits technically got made, but we somehow failed to parse response,
            // in this case terminate the proxy with 0 exit code so it wouldn't get automatically restarted
            // because it requires some serious MANUAL intervention
            error!(
                "CRITICAL FAILURE: failed to parse out deposit information from the contract transaction. either the chain got upgraded and the schema changed or the ecash contract got changed! terminating the process. it has to be inspected manually. error was: {err}"
            );
            cancellation_on_critical_failure.cancel();
            return Err(CredentialProxyError::DepositFailure);
        }
    };

    if contract_data.len() != amount {
        // another critical failure, that one should be quite impossible and thus has to be manually inspected
        error!(
            "CRITICAL FAILURE: failed to parse out all deposit information from the contract transaction. got {} responses while we sent {amount} deposits! either the chain got upgraded and the schema changed or the ecash contract got changed! terminating the process. it has to be inspected manually",
            contract_data.len()
        );
        cancellation_on_critical_failure.cancel();
        return Err(CredentialProxyError::DepositFailure);
    }

    let mut deposits_data = Vec::new();
    for (key, response) in keys.into_iter().zip(contract_data) {
        let response_index = response.message_index;
        let deposit_id = match response.parse_singleton_u32_contract_data() {
            Ok(deposit_id) => deposit_id,
            Err(err) => {
                // another impossibility
                error!(
                    "CRITICAL FAILURE: failed to parse out deposit id out of the response at index {response_index}: {err}. either the chain got upgraded and the schema changed or the ecash contract got changed! terminating the process. it has to be inspected manually"
                );
                cancellation_on_critical_failure.cancel();
                return Err(CredentialProxyError::DepositFailure);
            }
        };

        deposits_data.push(BufferedDeposit::new(deposit_id, key));
    }

    Ok(PerformedDeposits {
        deposits_data,
        tx_hash,
        requested_on,
        deposit_amount,
    })
}

pub fn split_deposits(total: usize, max_request_size: usize) -> impl Iterator<Item = usize> {
    (0..total)
        .step_by(max_request_size)
        .map(move |start| std::cmp::min(max_request_size, total - start))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_sizes_test() {
        assert_eq!(
            split_deposits(100, 32).collect::<Vec<_>>(),
            vec![32, 32, 32, 4]
        );

        assert_eq!(split_deposits(10, 32).collect::<Vec<_>>(), vec![10]);
        assert_eq!(split_deposits(32, 32).collect::<Vec<_>>(), vec![32]);
        assert_eq!(split_deposits(33, 32).collect::<Vec<_>>(), vec![32, 1]);
        assert_eq!(split_deposits(1, 32).collect::<Vec<_>>(), vec![1]);
    }
}
