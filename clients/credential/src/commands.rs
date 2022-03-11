// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use clap::{Args, Subcommand};
use pickledb::PickleDb;
use rand::rngs::OsRng;
use std::str::FromStr;
use url::Url;

use coconut_interface::{hash_to_scalar, Parameters};
use credentials::coconut::bandwidth::{
    obtain_signature, BandwidthVoucherAttributes, TOTAL_ATTRIBUTES,
};
use crypto::asymmetric::{encryption, identity};

use crate::client::Client;
use crate::error::{CredentialClientError, Result};
use crate::state::{KeyPair, State};
use crate::{DEPOSITS_KEY, SIGNER_AUTHORITIES};

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

        let mut states = db.get::<Vec<State>>(DEPOSITS_KEY).unwrap_or(vec![]);

        let client = Client::new();
        let tx_hash = client
            .deposit(
                self.amount,
                signing_keypair.public_key.clone(),
                encryption_keypair.public_key.clone(),
            )
            .await?;

        let state = State {
            amount: self.amount,
            tx_hash,
            signing_keypair,
            encryption_keypair,
        };
        states.push(state);
        db.set(DEPOSITS_KEY, &states).unwrap();

        Ok(())
    }
}

#[derive(Args, Clone)]
pub(crate) struct ListDeposits {}

#[async_trait]
impl Execute for ListDeposits {
    async fn execute(&self, db: &mut PickleDb) -> Result<()> {
        let states: Vec<String> = db
            .get::<Vec<State>>(DEPOSITS_KEY)
            .unwrap_or(vec![])
            .into_iter()
            .map(|state| state.tx_hash)
            .collect();
        println!("Hashes for available deposits: {:?}", states);

        Ok(())
    }
}

#[derive(Args, Clone)]
pub(crate) struct GetCredential {
    /// The hash of a successful deposit transaction
    #[clap(long)]
    tx_hash: String,
}

#[async_trait]
impl Execute for GetCredential {
    async fn execute(&self, db: &mut PickleDb) -> Result<()> {
        let state = db
            .get::<Vec<State>>(DEPOSITS_KEY)
            .ok_or(CredentialClientError::NoDeposit)?
            .into_iter()
            .find(|state| state.tx_hash == self.tx_hash)
            .ok_or(CredentialClientError::NoDeposit)?;
        let urls = SIGNER_AUTHORITIES.map(|addr| Url::from_str(addr).unwrap());

        let params = Parameters::new(TOTAL_ATTRIBUTES).unwrap();
        let bandwidth_credential_attributes = BandwidthVoucherAttributes {
            serial_number: params.random_scalar(),
            binding_number: params.random_scalar(),
            voucher_value: hash_to_scalar(state.amount.to_be_bytes()),
            voucher_info: hash_to_scalar(String::from("BandwidthVoucher").as_bytes()),
        };
        let _signature = obtain_signature(&params, &bandwidth_credential_attributes, &urls).await?;
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
