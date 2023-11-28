// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ephemera::client::Client;
use crate::ephemera::peers::NymPeer;
use crate::support::nyxd;
use cosmwasm_std::Addr;
use ephemera::membership::{PeerInfo, ProviderError};
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use nym_ephemera_common::types::JsonPeerInfo;
use std::future::Future;
use std::pin::Pin;
use std::task::Poll::Pending;
use std::task::{Context, Poll};

/// Future type which allows user to implement their own peers membership source mechanism.
pub type ProviderFut = BoxFuture<'static, Result<Vec<PeerInfo>, ProviderError>>;

///[`ProviderFut`] that reads peers from a http endpoint.
///
/// The endpoint must return a json array of [`JsonPeerInfo`].
/// # Configuration example
/// ```json
/// [
///  {
///     "name": "node1",
///     "address": "/ip4/",
///     "public_key": "4XTTMEghav9LZThm6opUaHrdGEEYUkrfkakVg4VAetetBZDWJ"
///   },
///  {
///     "name": "node2",
///     "address": "/ip4/",
///     "public_key": "4XTTMFQt2tgNRmwRgEAaGQe2NXygsK6Vr3pkuBfYezhDfoVty"
///   }
/// ]
/// ```
pub struct MembersProvider {
    /// The url of the http endpoint.
    nyxd_client: nyxd::Client,
    fut: Option<ProviderFut>,
}

impl MembersProvider {
    #[must_use]
    pub(crate) fn new(nyxd_client: nyxd::Client) -> Self {
        Self {
            nyxd_client,
            fut: None,
        }
    }

    pub(crate) async fn register_peer(
        &self,
        peer_info: NymPeer,
    ) -> crate::ephemera::error::Result<()> {
        let json_peer_info = JsonPeerInfo::new(
            Addr::unchecked(peer_info.cosmos_address),
            peer_info.ip_address,
            peer_info.public_key.to_string(),
        );
        self.nyxd_client
            .register_ephemera_peer(json_peer_info)
            .await?;
        Ok(())
    }

    async fn request_peers(nyxd_client: nyxd::Client) -> Result<Vec<PeerInfo>, ProviderError> {
        let peers = nyxd_client
            .get_ephemera_peers()
            .await
            .map_err(|e| ProviderError::GetPeers(e.to_string()))?
            .into_iter()
            .filter_map(|peer| PeerInfo::try_from(peer).ok())
            .collect();
        Ok(peers)
    }
}

impl Future for MembersProvider {
    type Output = Result<Vec<PeerInfo>, ProviderError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.fut.take() {
            None => {
                self.fut = Some(Box::pin(MembersProvider::request_peers(
                    self.nyxd_client.clone(),
                )));
            }
            Some(mut fut) => {
                let peers = match fut.poll_unpin(cx) {
                    Poll::Ready(Ok(peers)) => peers,
                    Poll::Ready(Err(err)) => {
                        error!("Failed to get peers: {err}");
                        return Poll::Ready(Err(err));
                    }
                    Pending => {
                        self.fut = Some(fut);
                        return Pending;
                    }
                };

                return Poll::Ready(Ok(peers));
            }
        }
        Pending
    }
}
