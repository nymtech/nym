// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::CredentialProxyError;
use crate::storage::models::StorableEcashDeposit;
use nym_compact_ecash::WithdrawalRequest;
use nym_credentials::IssuanceTicketBook;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::{Coin, Hash};
use time::OffsetDateTime;
use zeroize::Zeroizing;

pub(crate) struct BufferedDeposit {
    pub(crate) deposit_id: u32,

    // note: this type implements `ZeroizeOnDrop`
    pub(crate) ed25519_private_key: ed25519::PrivateKey,
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
    pub(crate) fn new(deposit_id: u32, ed25519_private_key: ed25519::PrivateKey) -> Self {
        BufferedDeposit {
            deposit_id,
            ed25519_private_key,
        }
    }

    pub(crate) fn sign_ticketbook_plaintext(
        &self,
        withdrawal_request: &WithdrawalRequest,
    ) -> ed25519::Signature {
        let plaintext = IssuanceTicketBook::request_plaintext(withdrawal_request, self.deposit_id);
        self.ed25519_private_key.sign(plaintext)
    }
}

pub(crate) struct PerformedDeposits {
    pub(crate) deposits_data: Vec<BufferedDeposit>,

    // shared by all performed deposits as they were included in the same tx
    pub(crate) tx_hash: Hash,
    pub(crate) requested_on: OffsetDateTime,
    pub(crate) deposit_amount: Coin,
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

pub(super) fn request_sizes(total: usize, max_request_size: usize) -> impl Iterator<Item = usize> {
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
            request_sizes(100, 32).collect::<Vec<_>>(),
            vec![32, 32, 32, 4]
        );

        assert_eq!(request_sizes(10, 32).collect::<Vec<_>>(), vec![10]);
        assert_eq!(request_sizes(32, 32).collect::<Vec<_>>(), vec![32]);
        assert_eq!(request_sizes(33, 32).collect::<Vec<_>>(), vec![32, 1]);
        assert_eq!(request_sizes(1, 32).collect::<Vec<_>>(), vec![1]);
    }
}
