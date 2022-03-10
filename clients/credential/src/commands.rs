// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use clap::{Args, Subcommand};
use pickledb::PickleDb;
use rand::rngs::OsRng;

use crypto::asymmetric::{encryption, identity};

use crate::client::Client;
use crate::state::{KeyPair, State};

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Deposit funds for buying coconut credential
    Deposit(Deposit),
    /// Get a credential for a given deposit
    GetCredential(GetCredential),
}

#[async_trait]
pub(crate) trait Execute {
    async fn execute(&self, db: &mut PickleDb);
}

#[derive(Args, Clone)]
pub(crate) struct Deposit {
    /// The amount that needs to be deposited
    #[clap(long)]
    amount: usize,
}

#[async_trait]
impl Execute for Deposit {
    async fn execute(&self, db: &mut PickleDb) {
        let mut rng = OsRng;
        let signing_keypair = KeyPair::from(identity::KeyPair::new(&mut rng));
        let encryption_keypair = KeyPair::from(encryption::KeyPair::new(&mut rng));

        let client = Client::new();

        let state = State {
            signing_keypair,
            encryption_keypair,
        };
        db.set("000", &state).unwrap();
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
    async fn execute(&self, _db: &mut PickleDb) {}
}

#[derive(Args, Clone)]
pub(crate) struct SpendCredential {
    /// Spend one of the acquired credentials
    #[clap(long)]
    id: usize,
}

#[async_trait]
impl Execute for SpendCredential {
    async fn execute(&self, _db: &mut PickleDb) {}
}
