// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::{Error, Result};
use nym_bandwidth_controller::acquire::state::State;
use nym_credential_storage::storage::Storage;
use nym_credentials::coconut::bandwidth::IssuanceTicketBook;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use zeroize::Zeroizing;

/// The serialized version of the yet untransformed bandwidth voucher. It can be used to complete
/// the acquirement process of a bandwidth credential.
/// Its serialized nature makes it easy to store and load it to e.g. disk.
pub type VoucherBlob = Vec<u8>;

/// Represents a client that can be used to acquire bandwidth. You typically create one when you
/// want to connect to the mixnet using paid coconut bandwidth credentials.
/// The way to create this client is by calling
/// [`crate::mixnet::DisconnectedMixnetClient::create_bandwidth_client`] on the associated mixnet
/// client.
pub struct BandwidthAcquireClient<'a, St: Storage> {
    client: DirectSigningHttpRpcNyxdClient,
    storage: &'a St,
    client_id: Zeroizing<String>,
}

impl<'a, St> BandwidthAcquireClient<'a, St>
where
    St: Storage,
    <St as Storage>::StorageError: Send + Sync + 'static,
{
    pub(crate) fn new(
        network_details: NymNetworkDetails,
        mnemonic: String,
        storage: &'a St,
        client_id: String,
    ) -> Result<Self> {
        let nyxd_url = network_details.endpoints[0].nyxd_url.as_str();
        let config = nyxd::Config::try_from_nym_network_details(&network_details)?;

        let client = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            config,
            nyxd_url,
            mnemonic.parse()?,
        )?;
        Ok(Self {
            client,
            storage,
            client_id: client_id.into(),
        })
    }

    /// Buy a credential worth amount utokens. If [`Error::UnconvertedDeposit`] is returned, it
    /// means the tokens have been deposited, but the proper bandwidth credential hasn't yet been
    /// created. A [`VoucherBlob`] is returned that can be used for a later recovery of the
    /// associated bandwidth credential, using [`Self::recover`].
    pub async fn acquire(&self) -> Result<()> {
        let state =
            nym_bandwidth_controller::acquire::deposit(&self.client, self.client_id.as_bytes())
                .await?;
        nym_bandwidth_controller::acquire::get_bandwidth_voucher(&state, &self.client, self.storage)
            .await
            .map_err(|reason| Error::UnconvertedDeposit {
                reason,
                voucher_blob: state.voucher.to_recovery_bytes(),
            })
    }

    /// In case of an error in the mid of the acquire process, this function should be used for
    /// later retries to recover the bandwidth credential, either immediately or after some time.
    pub async fn recover(&self, voucher_blob: &VoucherBlob) -> Result<()> {
        let voucher = IssuanceTicketBook::try_from_recovered_bytes(voucher_blob)
            .map_err(|_| Error::InvalidVoucherBlob)?;
        let state = State::new(voucher);
        nym_bandwidth_controller::acquire::get_bandwidth_voucher(
            &state,
            &self.client,
            self.storage,
        )
        .await?;

        Ok(())
    }
}
