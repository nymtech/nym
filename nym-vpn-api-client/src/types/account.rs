// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::fmt;

use nym_compact_ecash::scheme::keygen::KeyPairUser;
use nym_validator_client::{
    DirectSecp256k1HdWallet, nyxd::bip32::DerivationPath, signing::signer::OfflineSigner as _,
};
use time::{Duration, OffsetDateTime};

use crate::{VpnApiClientError, error::Result, jwt::Jwt};

const MAX_ACCEPTABLE_SKEW_SECONDS: i64 = 60;
const SKEW_SECONDS_CONSIDERED_SAME: i64 = 2;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("wallet error")]
    Wallet(#[from] nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWalletError),

    #[error("no accounts in wallet")]
    NoAccounts,
}

#[derive(Clone, Debug)]
pub struct VpnApiAccount {
    wallet: DirectSecp256k1HdWallet,
    id: String,
    pub_key: String,
    signature_base64: String,
}

impl VpnApiAccount {
    fn derive_from_wallet(wallet: DirectSecp256k1HdWallet) -> std::result::Result<Self, Error> {
        let accounts = wallet.get_accounts()?;
        let address = accounts.first().ok_or(Error::NoAccounts)?.address();
        let id = address.to_string();
        let pub_key = bs58::encode(
            accounts
                .first()
                .ok_or(Error::NoAccounts)?
                .public_key()
                .to_bytes(),
        )
        .into_string();

        let message = id.clone().into_bytes();
        let signature = wallet.sign_raw(address, message)?;
        let signature_bytes = signature.to_bytes().to_vec();
        let signature_base64 = base64_url::encode(&signature_bytes);

        Ok(Self {
            wallet,
            id,
            pub_key,
            signature_base64,
        })
    }

    pub fn random() -> Result<(Self, bip39::Mnemonic)> {
        let mnemonic = bip39::Mnemonic::generate(24).unwrap();
        let wallet = DirectSecp256k1HdWallet::from_mnemonic("n", mnemonic.clone());
        let account = Self::derive_from_wallet(wallet).map_err(VpnApiClientError::CreateAccount)?;
        Ok((account, mnemonic))
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn pub_key(&self) -> &str {
        &self.pub_key
    }

    pub fn signature_base64(&self) -> &str {
        &self.signature_base64
    }

    pub(crate) fn jwt(&self, remote_time: Option<VpnApiTime>) -> Jwt {
        match remote_time {
            Some(remote_time) => Jwt::new_secp256k1_synced(&self.wallet, remote_time),
            None => Jwt::new_secp256k1(&self.wallet),
        }
    }

    pub fn create_ecash_keypair(&self) -> Result<KeyPairUser> {
        let hd_path = cosmos_derivation_path();
        let extended_private_key = self
            .wallet
            .derive_extended_private_key(&hd_path)
            .map_err(VpnApiClientError::CosmosDeriveFromPath)?;
        Ok(KeyPairUser::new_seeded(
            extended_private_key.private_key().to_bytes(),
        ))
    }

    pub fn get_mnemonic(&self) -> String {
        self.wallet.mnemonic()
    }
}

impl TryFrom<bip39::Mnemonic> for VpnApiAccount {
    type Error = VpnApiClientError;

    fn try_from(mnemonic: bip39::Mnemonic) -> std::result::Result<Self, Self::Error> {
        let wallet = DirectSecp256k1HdWallet::from_mnemonic("n", mnemonic.clone());
        Self::derive_from_wallet(wallet).map_err(VpnApiClientError::CreateAccount)
    }
}

fn cosmos_derivation_path() -> DerivationPath {
    nym_config::defaults::COSMOS_DERIVATION_PATH
        .parse()
        .unwrap()
}

#[derive(Clone, Copy, Debug)]
pub struct VpnApiTime {
    // The local time on the client.
    pub local_time: OffsetDateTime,

    // The estimated time on the remote server. Based on RTT, it's not guaranteed to be accurate.
    pub estimated_remote_time: OffsetDateTime,
}

impl VpnApiTime {
    pub fn from_estimated_remote_time(
        local_time: OffsetDateTime,
        estimated_remote_time: OffsetDateTime,
    ) -> Self {
        Self {
            local_time,
            estimated_remote_time,
        }
    }

    pub fn from_remote_timestamp(
        local_time_before_request: OffsetDateTime,
        remote_timestamp: OffsetDateTime,
        local_time_after_request: OffsetDateTime,
    ) -> Self {
        let rtt = local_time_after_request - local_time_before_request;
        let estimated_remote_time = remote_timestamp + (rtt / 2);
        Self {
            local_time: local_time_after_request,
            estimated_remote_time,
        }
    }

    // Local time minus remote time. Meaning if the value is positive, the local time is ahead
    // of the remote time.
    pub fn local_time_ahead_skew(&self) -> Duration {
        self.local_time - self.estimated_remote_time
    }

    pub fn is_almost_same(&self) -> bool {
        self.local_time_ahead_skew().abs().whole_seconds() < SKEW_SECONDS_CONSIDERED_SAME
    }

    pub fn is_acceptable_synced(&self) -> bool {
        self.local_time_ahead_skew().abs().whole_seconds() < MAX_ACCEPTABLE_SKEW_SECONDS
    }

    pub fn is_synced(&self) -> VpnApiTimeSynced {
        if self.is_almost_same() {
            VpnApiTimeSynced::AlmostSame
        } else if self.is_acceptable_synced() {
            VpnApiTimeSynced::AcceptableSynced
        } else {
            VpnApiTimeSynced::NotSynced
        }
    }

    pub fn estimate_remote_now(&self) -> OffsetDateTime {
        tracing::debug!(
            "Estimating remote now using (local time ahead) skew: {}",
            self.local_time_ahead_skew()
        );
        let local_time_now = OffsetDateTime::now_utc();
        local_time_now - self.local_time_ahead_skew()
    }

    pub fn estimate_remote_now_unix(&self) -> u128 {
        self.estimate_remote_now().unix_timestamp() as u128
    }
}

impl fmt::Display for VpnApiTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Local time: {}, Remote time: {}, Skew: {}",
            self.local_time,
            self.estimated_remote_time,
            self.local_time_ahead_skew(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VpnApiTimeSynced {
    AlmostSame,
    AcceptableSynced,
    NotSynced,
}

impl VpnApiTimeSynced {
    pub fn is_synced(&self) -> bool {
        matches!(
            self,
            VpnApiTimeSynced::AlmostSame | VpnApiTimeSynced::AcceptableSynced
        )
    }

    pub fn is_not_synced(&self) -> bool {
        !self.is_synced()
    }
}

#[cfg(test)]
mod tests {
    use crate::types::test_fixtures::{TEST_DEFAULT_MNEMONIC, TEST_DEFAULT_MNEMONIC_ID};

    use super::*;

    #[test]
    fn create_account_from_mnemonic() {
        let account =
            VpnApiAccount::try_from(bip39::Mnemonic::parse(TEST_DEFAULT_MNEMONIC).unwrap())
                .unwrap();
        assert_eq!(account.id(), TEST_DEFAULT_MNEMONIC_ID);
    }

    #[test]
    fn create_random_account() {
        let (_, mnemonic) = VpnApiAccount::random().unwrap();
        assert_eq!(mnemonic.word_count(), 24);
    }

    #[test]
    fn derive_wallets() {
        for word_count in [12, 24] {
            let wallet = DirectSecp256k1HdWallet::generate("n", word_count).unwrap();
            VpnApiAccount::derive_from_wallet(wallet).unwrap();
        }
    }
}
