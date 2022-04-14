// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use clap::{Args, Subcommand};
use pickledb::PickleDb;
use rand::rngs::OsRng;
use std::str::FromStr;
use url::Url;

use coconut_interface::{Attribute, Base58, BlindSignRequest, Bytable, Parameters};
use credential_storage::storage::Storage;
use credential_storage::PersistentStorage;
use credentials::coconut::bandwidth::{BandwidthVoucher, TOTAL_ATTRIBUTES};
use credentials::coconut::utils::obtain_aggregate_signature;
use crypto::asymmetric::{encryption, identity};
use network_defaults::VOUCHER_INFO;
use validator_client::nymd::tx::Hash;

use crate::client::Client;
use crate::error::{CredentialClientError, Result};
use crate::state::{KeyPair, RequestData, State};
use crate::SIGNER_AUTHORITIES;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Deposit funds for buying coconut credential
    Deposit(Deposit),
    /// Lists the tx hashes of previous deposits
    ListDeposits(ListDeposits),
    /// Get a credential for a given deposit
    GetCredential(GetCredential),
}

#[async_trait]
pub(crate) trait Execute {
    async fn execute(&self, db: &mut PickleDb, shared_storage: PersistentStorage) -> Result<()>;
}

#[derive(Args, Clone)]
pub(crate) struct Deposit {
    /// The amount that needs to be deposited
    #[clap(long)]
    amount: u64,
}

#[async_trait]
impl Execute for Deposit {
    async fn execute(&self, db: &mut PickleDb, _shared_storage: PersistentStorage) -> Result<()> {
        let mut rng = OsRng;
        let signing_keypair = KeyPair::from(identity::KeyPair::new(&mut rng));
        let encryption_keypair = KeyPair::from(encryption::KeyPair::new(&mut rng));

        let client = Client::new();
        let tx_hash = client
            .deposit(
                self.amount,
                VOUCHER_INFO,
                signing_keypair.public_key.clone(),
                encryption_keypair.public_key.clone(),
            )
            .await?;

        let state = State {
            amount: self.amount,
            tx_hash: tx_hash.clone(),
            signing_keypair,
            encryption_keypair,
            blind_request_data: None,
            signature: None,
        };
        db.set(&tx_hash, &state).unwrap();

        println!("{:?}", state);

        Ok(())
    }
}

#[derive(Args, Clone)]
pub(crate) struct ListDeposits {}

#[async_trait]
impl Execute for ListDeposits {
    async fn execute(&self, db: &mut PickleDb, _shared_storage: PersistentStorage) -> Result<()> {
        for kv in db.iter() {
            println!("{:?}", kv.get_value::<State>());
        }

        Ok(())
    }
}

#[derive(Args, Clone)]
pub(crate) struct GetCredential {
    /// The hash of a successful deposit transaction
    #[clap(long)]
    tx_hash: String,
    /// If we want to get the signature without attaching a blind sign request; it is expected that
    /// there is already a signature stored on the signer
    #[clap(long, parse(from_flag))]
    __no_request: bool,
}

#[async_trait]
impl Execute for GetCredential {
    async fn execute(&self, db: &mut PickleDb, shared_storage: PersistentStorage) -> Result<()> {
        let mut state = db
            .get::<State>(&self.tx_hash)
            .ok_or(CredentialClientError::NoDeposit)?;
        let urls = SIGNER_AUTHORITIES.map(|addr| Url::from_str(addr).unwrap());

        let params = Parameters::new(TOTAL_ATTRIBUTES).unwrap();
        let bandwidth_credential_attributes = if self.__no_request {
            if let Some(blind_request_data) = state.blind_request_data {
                let serial_number =
                    Attribute::try_from_byte_slice(&blind_request_data.serial_number)
                        .map_err(|_| CredentialClientError::CorruptedBlindSignRequest)?;
                let binding_number =
                    Attribute::try_from_byte_slice(&blind_request_data.binding_number)
                        .map_err(|_| CredentialClientError::CorruptedBlindSignRequest)?;
                let pedersen_commitments_openings = vec![
                    Attribute::try_from_byte_slice(&blind_request_data.first_attribute)
                        .map_err(|_| CredentialClientError::CorruptedBlindSignRequest)?,
                    Attribute::try_from_byte_slice(&blind_request_data.second_attribute)
                        .map_err(|_| CredentialClientError::CorruptedBlindSignRequest)?,
                ];
                let blind_sign_request =
                    BlindSignRequest::from_bytes(blind_request_data.blind_sign_req.as_slice())
                        .map_err(|_| CredentialClientError::CorruptedBlindSignRequest)?;
                BandwidthVoucher::new_with_blind_sign_req(
                    [serial_number, binding_number],
                    [&state.amount.to_string(), VOUCHER_INFO],
                    Hash::from_str(&self.tx_hash)
                        .map_err(|_| CredentialClientError::InvalidTxHash)?,
                    identity::PrivateKey::from_base58_string(&state.signing_keypair.private_key)?,
                    encryption::PrivateKey::from_base58_string(
                        &state.encryption_keypair.private_key,
                    )?,
                    pedersen_commitments_openings,
                    blind_sign_request,
                )
            } else {
                return Err(CredentialClientError::NoLocalBlindSignRequest);
            }
        } else {
            BandwidthVoucher::new(
                &params,
                state.amount.to_string(),
                VOUCHER_INFO.to_string(),
                Hash::from_str(&self.tx_hash).map_err(|_| CredentialClientError::InvalidTxHash)?,
                identity::PrivateKey::from_base58_string(&state.signing_keypair.private_key)?,
                encryption::PrivateKey::from_base58_string(&state.encryption_keypair.private_key)?,
            )
        };

        // Back up the blind sign req data, in case of sporadic failures
        state.blind_request_data = Some(RequestData::new(
            bandwidth_credential_attributes.get_private_attributes(),
            bandwidth_credential_attributes.pedersen_commitments_openings(),
            bandwidth_credential_attributes.blind_sign_request(),
        )?);
        db.set(&self.tx_hash, &state).unwrap();

        let signature =
            obtain_aggregate_signature(&params, &bandwidth_credential_attributes, &urls).await?;
        shared_storage
            .insert_coconut_credential(
                state.amount.to_string(),
                VOUCHER_INFO.to_string(),
                bandwidth_credential_attributes.get_private_attributes()[0].to_bs58(),
                bandwidth_credential_attributes.get_private_attributes()[1].to_bs58(),
                signature.to_bs58(),
            )
            .await?;
        state.signature = Some(signature.to_bs58());
        db.set(&self.tx_hash, &state).unwrap();

        println!("Signature: {:?}", state.signature);

        Ok(())
    }
}
