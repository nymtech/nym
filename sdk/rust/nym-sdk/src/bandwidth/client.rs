// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::error::Result;
use nym_bandwidth_controller::{error::BandwidthControllerError, BandwidthController};
use nym_bandwidth_fetcher::NyxdCredentialFetcher;
use nym_credential_storage::storage::Storage;
use nym_credentials_interface::TicketType;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use zeroize::Zeroizing;

/// Represents a client that can be used to acquire bandwidth.
///
/// Represents a client that can be used to acquire bandwidth. You typically create one when you
/// want to connect to the mixnet using paid coconut bandwidth credentials.
/// The way to create this client is by calling
/// [`crate::mixnet::DisconnectedMixnetClient::create_bandwidth_client`] on the associated mixnet
/// client.
pub struct BandwidthAcquireClient<St: Storage> {
    bandwidth_controller: BandwidthController<St>,
    ticketbook_type: TicketType,
}

impl<St> BandwidthAcquireClient<St>
where
    St: Storage + 'static,
{
    #[allow(clippy::result_large_err)]
    pub(crate) async fn new(
        network_details: NymNetworkDetails,
        mnemonic: String,
        storage: St,
        client_id: Vec<u8>,
        ticketbook_type: TicketType,
    ) -> Result<Self> {
        let nyxd_url = network_details.endpoints[0].nyxd_url.as_str();
        let config = nyxd::Config::try_from_nym_network_details(&network_details)?;

        let client = Arc::new(DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            config,
            nyxd_url,
            mnemonic.parse()?,
        )?);

        let credential_fetcher =
            NyxdCredentialFetcher::new(client, ":memory:", Zeroizing::new(client_id))
                .await
                .map_err(|e| BandwidthControllerError::fetcher_error(Box::new(e)))?;
        let bandwidth_controller =
            BandwidthController::new(storage.clone()).with_credential_fetcher(credential_fetcher);

        Ok(Self {
            bandwidth_controller,
            ticketbook_type,
        })
    }

    pub async fn acquire(&self) -> Result<()> {
        self.bandwidth_controller
            .fetch_ticketbook(self.ticketbook_type)
            .await?;
        Ok(())
    }
}
