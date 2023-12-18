// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use nym_compact_ecash::scheme::keygen::KeyPairUser;
use nym_compact_ecash::scheme::{EcashCredential, Wallet};
use nym_compact_ecash::setup::{setup, Parameters};
use nym_compact_ecash::{Base58, PayInfo};
use nym_credential_storage::error::StorageError;
use nym_credential_storage::storage::Storage;
use nym_credentials::obtain_aggregate_verification_key;
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use std::str::FromStr;

pub mod acquire;
pub mod error;

pub struct BandwidthController<C, St> {
    storage: St,
    client: C,
    ecash_keypair: KeyPairUser,
    ecash_params: Parameters,
}

impl<C, St: Storage> BandwidthController<C, St> {
    pub fn new(
        storage: St,
        client: C,
        ecash_keypair: KeyPairUser,
        ecash_params: Parameters,
    ) -> Self {
        BandwidthController {
            storage,
            client,
            ecash_keypair,
            ecash_params,
        }
    }

    pub fn storage(&self) -> &St {
        &self.storage
    }

    pub async fn prepare_ecash_credential(
        &self,
        provider_pk: [u8; 32],
    ) -> Result<(EcashCredential, Wallet, i64), BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        let ecash_wallet = self
            .storage
            .get_next_ecash_wallet()
            .await
            .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))?;

        let wallet = Wallet::try_from_bs58(ecash_wallet.wallet)?;
        let epoch_id =
            u64::from_str(&ecash_wallet.epoch_id).map_err(|_| StorageError::InconsistentData)?;

        let ecash_api_clients = all_ecash_api_clients(&self.client, epoch_id).await?;

        let verification_key = obtain_aggregate_verification_key(&ecash_api_clients).await?;

        let sk_user = self.ecash_keypair.secret_key();
        let pay_info = PayInfo::generate_payinfo(provider_pk);
        let nb_tickets = 1u64; //SW: TEMPORARY VALUE, what should we put there?
        let wallet_value = u64::from_str(&ecash_wallet.value)
            .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))?;
        let credential_value = nb_tickets * wallet_value / (self.ecash_params.ll());

        // the below would only be executed once we know where we want to spend it (i.e. which gateway and stuff)

        let (payment, _) = wallet.spend(
            &self.ecash_params,
            &verification_key,
            &sk_user,
            &pay_info,
            false,
            nb_tickets,
        )?;

        let credential = EcashCredential::new(payment, credential_value, pay_info, epoch_id);

        Ok((credential, wallet, ecash_wallet.id))
    }

    pub async fn update_ecash_wallet(
        &self,
        wallet: Wallet,
        id: i64,
    ) -> Result<(), BandwidthControllerError>
    where
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        // JS: shouldn't we send some contract/validator/gateway message here to actually, you know,
        // consume it?
        let consumed = wallet.l() >= setup(100).ll(); //temporary, depends on parameters distribution
        let wallet_string = wallet.to_bs58();

        self.storage
            .update_ecash_wallet(wallet_string, id, consumed)
            .await
            .map_err(|err| BandwidthControllerError::CredentialStorageError(Box::new(err)))
    }
}

impl<C, St> Clone for BandwidthController<C, St>
where
    C: Clone,
    St: Storage + Clone,
{
    fn clone(&self) -> Self {
        BandwidthController {
            storage: self.storage.clone(),
            client: self.client.clone(),
            ecash_keypair: self.ecash_keypair.clone(),
            ecash_params: self.ecash_params.clone(),
        }
    }
}
