// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Result;
use nym_credential_storage::storage::Storage;
use nym_credential_utils::utils::issue_credential;
use nym_credentials_interface::TicketType;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use std::ops::Deref;
use zeroize::Zeroizing;

/// Represents a client that can be used to acquire bandwidth.
///
/// Represents a client that can be used to acquire bandwidth. You typically create one when you
/// want to connect to the mixnet using paid coconut bandwidth credentials.
/// The way to create this client is by calling
/// [`crate::mixnet::DisconnectedMixnetClient::create_bandwidth_client`] on the associated mixnet
/// client.
pub struct BandwidthAcquireClient<'a, St: Storage> {
    client: DirectSigningHttpRpcNyxdClient,
    storage: &'a St,
    client_id: Zeroizing<Vec<u8>>,
    ticketbook_type: TicketType,
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
        client_id: Vec<u8>,
        ticketbook_type: TicketType,
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
            ticketbook_type,
        })
    }

    pub async fn acquire(&self) -> Result<()> {
        issue_credential(
            &self.client,
            self.storage,
            self.client_id.deref(),
            self.ticketbook_type,
        )
        .await?;
        Ok(())
    }
}
