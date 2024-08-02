// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::client::Client as LocalClient;
use crate::ecash::comm::APICommunicationChannel;
use crate::ecash::error::{EcashError, Result};
use crate::support::storage::NymApiStorage;
use nym_coconut_dkg_common::types::EpochId;

pub(crate) struct AuxiliaryEcashState {
    pub(crate) client: Box<dyn LocalClient + Send + Sync>,
    pub(crate) comm_channel: Box<dyn APICommunicationChannel + Send + Sync>,
    pub(crate) storage: NymApiStorage,
}

impl AuxiliaryEcashState {
    pub(crate) fn new<C, D>(client: C, comm_channel: D, storage: NymApiStorage) -> Self
    where
        C: LocalClient + Send + Sync + 'static,
        D: APICommunicationChannel + Send + Sync + 'static,
    {
        AuxiliaryEcashState {
            client: Box::new(client),
            comm_channel: Box::new(comm_channel),
            storage,
        }
    }

    pub(crate) async fn current_epoch(&self) -> Result<EpochId> {
        self.comm_channel.current_epoch().await
    }

    pub(crate) async fn ensure_not_blacklisted(&self, encoded_pubkey_bs58: &str) -> Result<()> {
        let res = self
            .client
            .get_blacklisted_account(encoded_pubkey_bs58.to_string())
            .await?;

        if res.account.is_some() {
            return Err(EcashError::BlacklistedAccount);
        }

        Ok(())
    }
}
