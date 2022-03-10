// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Deposit funds for buying coconut credential
    Deposit(Deposit),
    GetCredential(GetCredential),
}

#[async_trait]
pub(crate) trait Execute {
    async fn execute(&self);
}

#[derive(Args, Clone)]
pub(crate) struct Deposit {
    /// The amount that needs to be deposited
    #[clap(long)]
    amount: String,
}

#[async_trait]
impl Execute for Deposit {
    async fn execute(&self) {}
}

#[derive(Args, Clone)]
pub(crate) struct GetCredential {
    /// The hash of a successful deposit transaction
    #[clap(long)]
    tx_hash: String,
}

#[async_trait]
impl Execute for GetCredential {
    async fn execute(&self) {}
}

#[derive(Args, Clone)]
pub(crate) struct SpendCredential {
    /// Spend one of the acquired credentials
    #[clap(long)]
    id: usize,
}

#[async_trait]
impl Execute for SpendCredential {
    async fn execute(&self) {}
}
