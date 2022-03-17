// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use clap::{Args, Subcommand};
use pickledb::PickleDb;
use rand::rngs::OsRng;
use std::str::FromStr;
use url::Url;

use coconut_interface::{Base58, Parameters};
use credentials::coconut::bandwidth::{BandwidthVoucher, TOTAL_ATTRIBUTES};
use credentials::coconut::utils::obtain_aggregate_signature;
use crypto::asymmetric::{encryption, identity};
use network_defaults::VOUCHER_INFO;

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
    async fn execute(&self, db: &mut PickleDb) -> Result<()>;
}

#[derive(Args, Clone)]
pub(crate) struct Deposit {
    /// The amount that needs to be deposited
    #[clap(long)]
    amount: u64,
}

#[async_trait]
impl Execute for Deposit {
    async fn execute(&self, db: &mut PickleDb) -> Result<()> {
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
    async fn execute(&self, db: &mut PickleDb) -> Result<()> {
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
    async fn execute(&self, db: &mut PickleDb) -> Result<()> {
        let mut state = db
            .get::<State>(&self.tx_hash)
            .ok_or(CredentialClientError::NoDeposit)?;
        let urls = SIGNER_AUTHORITIES.map(|addr| Url::from_str(addr).unwrap());

        let params = Parameters::new(TOTAL_ATTRIBUTES).unwrap();
        let bandwidth_credential_attributes = BandwidthVoucher::new(
            &params,
            &state.amount.to_string(),
            VOUCHER_INFO,
            self.tx_hash.clone(),
            state.signing_keypair.private_key.clone(),
            state.encryption_keypair.private_key.clone(),
        );

        // Back up the blind sign req data, in case of sporadic failures
        state.blind_request_data = Some(RequestData::new(
            &bandwidth_credential_attributes.pedersen_commitments_openings(),
            bandwidth_credential_attributes.blind_sign_request(),
        )?);
        db.set(&self.tx_hash, &state).unwrap();

        let signature =
            obtain_aggregate_signature(&params, &bandwidth_credential_attributes, &urls).await?;
        state.signature = Some(signature.to_bs58());
        db.set(&self.tx_hash, &state).unwrap();

        println!("Signature: {:?}", state.signature);

        Ok(())
    }
}

#[derive(Args, Clone)]
pub(crate) struct SpendCredential {
    /// Spend one of the acquired credentials
    #[clap(long)]
    id: usize,
}

#[async_trait]
impl Execute for SpendCredential {
    async fn execute(&self, _db: &mut PickleDb) -> Result<()> {
        Ok(())
    }
}
