// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_describe_cache::NodeDescribeCacheError;
use futures::future::{maybe_done, MaybeDone};
use futures::{FutureExt, TryFutureExt};
use nym_api_requests::models::{
    AuthenticatorDetails, HostInformation, IpPacketRouterDetails, NetworkRequesterDetails,
    NymNodeData, WebSockets, WireguardDetails,
};
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_config::defaults::mainnet;
use nym_mixnet_contract_common::NodeId;
use nym_node_requests::api::client::{NymNodeApiClientError, NymNodeApiClientExt};
use nym_node_requests::api::v1::node::models::{AuxiliaryDetails, NodeRoles};
use nym_node_requests::api::Client;
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use time::OffsetDateTime;
use tracing::debug;

async fn network_requester_future(
    client: &Client,
) -> Result<Option<NetworkRequesterDetails>, NymNodeApiClientError> {
    let Ok(nr) = client.get_network_requester().await else {
        return Ok(None);
    };

    client.get_exit_policy().await.map(|exit_policy| {
        let uses_nym_exit_policy = exit_policy.upstream_source == mainnet::EXIT_POLICY_URL;
        Some(NetworkRequesterDetails {
            address: nr.address,
            uses_exit_policy: exit_policy.enabled && uses_nym_exit_policy,
        })
    })
}

pub(crate) async fn query_for_described_data(
    client: &Client,
    node_id: NodeId,
) -> Result<UnwrappedResolvedNodeDescribedInfo, NodeDescribeCacheError> {
    let map_query_err = |source| NodeDescribeCacheError::ApiFailure { node_id, source };

    // all of those should be happening concurrently.
    NodeDescribedInfoMegaFuture::new(
        client.get_build_information().map_err(map_query_err),
        client.get_roles().map_err(map_query_err),
        client.get_auxiliary_details()
            .inspect_err(|err| {
                // old nym-nodes will not have this field, so use the default instead
                debug!("could not obtain auxiliary details of node {node_id}: {err} is it running an old version?")
            })
            .unwrap_or_else(|_| AuxiliaryDetails::default()),
        client.get_mixnet_websockets().ok_into().map_err(map_query_err),
        network_requester_future(client).map_err(map_query_err),
        // `ok_into` ultimately calls `IpPacketRouter::into` to transform it into `IpPacketRouterDetails`
        client.get_ip_packet_router().ok_into().map(Result::ok),
        client.get_authenticator().ok_into().map(Result::ok),
        client.get_wireguard().ok_into().map(Result::ok)
    )
        .await
}

// just a helper to have named fields as opposed to a mega tuple
// could I have used something more sophisticated? sure.
// is this code disgusting? yes. does it work? also yes
// (note: I've just mostly copied code from `futures-util::generate` macro where
// they derive code for `join2`, `join3`, etc.)
#[pin_project]
struct NodeDescribedInfoMegaFuture<F1, F2, F3, F4, F5, F6, F7, F8>
where
    F1: Future,
    F2: Future,
    F3: Future,
    F4: Future,
    F5: Future,
    F6: Future,
    F7: Future,
    F8: Future,
{
    #[pin]
    build_info: MaybeDone<F1>,
    #[pin]
    roles: MaybeDone<F2>,
    #[pin]
    auxiliary_details: MaybeDone<F3>,
    #[pin]
    websockets: MaybeDone<F4>,
    #[pin]
    network_requester: MaybeDone<F5>,
    #[pin]
    ipr: MaybeDone<F6>,
    #[pin]
    authenticator: MaybeDone<F7>,
    #[pin]
    wireguard: MaybeDone<F8>,
}

impl<F1, F2, F3, F4, F5, F6, F7, F8> Future
    for NodeDescribedInfoMegaFuture<F1, F2, F3, F4, F5, F6, F7, F8>
where
    F1: Future<Output = Result<BinaryBuildInformationOwned, NodeDescribeCacheError>>,
    F2: Future<Output = Result<NodeRoles, NodeDescribeCacheError>>,
    F3: Future<Output = AuxiliaryDetails>,
    F4: Future<Output = Result<WebSockets, NodeDescribeCacheError>>,
    F5: Future<Output = Result<Option<NetworkRequesterDetails>, NodeDescribeCacheError>>,
    F6: Future<Output = Option<IpPacketRouterDetails>>,
    F7: Future<Output = Option<AuthenticatorDetails>>,
    F8: Future<Output = Option<WireguardDetails>>,
{
    type Output = Result<UnwrappedResolvedNodeDescribedInfo, NodeDescribeCacheError>;

    // SAFETY: we've explicitly checked all futures have completed thus the unwraps are fine
    #[allow(clippy::unwrap_used)]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_done = true;
        let mut futures = self.project();

        all_done &= futures.build_info.as_mut().poll(cx).is_ready();
        all_done &= futures.roles.as_mut().poll(cx).is_ready();
        all_done &= futures.auxiliary_details.as_mut().poll(cx).is_ready();
        all_done &= futures.websockets.as_mut().poll(cx).is_ready();
        all_done &= futures.network_requester.as_mut().poll(cx).is_ready();
        all_done &= futures.ipr.as_mut().poll(cx).is_ready();
        all_done &= futures.authenticator.as_mut().poll(cx).is_ready();
        all_done &= futures.wireguard.as_mut().poll(cx).is_ready();

        if all_done {
            Poll::Ready(
                ResolvedNodeDescribedInfo {
                    build_info: futures.build_info.take_output().unwrap(),
                    roles: futures.roles.take_output().unwrap(),
                    auxiliary_details: futures.auxiliary_details.take_output().unwrap(),
                    websockets: futures.websockets.take_output().unwrap(),
                    network_requester: futures.network_requester.take_output().unwrap(),
                    ipr: futures.ipr.take_output().unwrap(),
                    authenticator: futures.authenticator.take_output().unwrap(),
                    wireguard: futures.wireguard.take_output().unwrap(),
                }
                .try_unwrap(),
            )
        } else {
            Poll::Pending
        }
    }
}

impl<F1, F2, F3, F4, F5, F6, F7, F8> NodeDescribedInfoMegaFuture<F1, F2, F3, F4, F5, F6, F7, F8>
where
    F1: Future,
    F2: Future,
    F3: Future,
    F4: Future,
    F5: Future,
    F6: Future,
    F7: Future,
    F8: Future,
{
    // okay. the fact I have to bypass clippy here means it wasn't a good idea to create this abomination after all
    #[allow(clippy::too_many_arguments)]
    fn new(
        build_info: F1,
        roles: F2,
        auxiliary_details: F3,
        websockets: F4,
        network_requester: F5,
        ipr: F6,
        authenticator: F7,
        wireguard: F8,
    ) -> Self {
        NodeDescribedInfoMegaFuture {
            build_info: maybe_done(build_info),
            roles: maybe_done(roles),
            auxiliary_details: maybe_done(auxiliary_details),
            websockets: maybe_done(websockets),
            network_requester: maybe_done(network_requester),
            ipr: maybe_done(ipr),
            authenticator: maybe_done(authenticator),
            wireguard: maybe_done(wireguard),
        }
    }
}

struct ResolvedNodeDescribedInfo {
    build_info: Result<BinaryBuildInformationOwned, NodeDescribeCacheError>,
    roles: Result<NodeRoles, NodeDescribeCacheError>,
    // TODO: in the future make it return a Result as well.
    auxiliary_details: AuxiliaryDetails,
    websockets: Result<WebSockets, NodeDescribeCacheError>,
    network_requester: Result<Option<NetworkRequesterDetails>, NodeDescribeCacheError>,
    ipr: Option<IpPacketRouterDetails>,
    authenticator: Option<AuthenticatorDetails>,
    wireguard: Option<WireguardDetails>,
}

impl ResolvedNodeDescribedInfo {
    fn try_unwrap(self) -> Result<UnwrappedResolvedNodeDescribedInfo, NodeDescribeCacheError> {
        Ok(UnwrappedResolvedNodeDescribedInfo {
            build_info: self.build_info?,
            roles: self.roles?,
            auxiliary_details: self.auxiliary_details,
            websockets: self.websockets?,
            network_requester: self.network_requester?,
            ipr: self.ipr,
            authenticator: self.authenticator,
            wireguard: self.wireguard,
        })
    }
}

pub(crate) struct UnwrappedResolvedNodeDescribedInfo {
    pub(crate) build_info: BinaryBuildInformationOwned,
    pub(crate) roles: NodeRoles,
    pub(crate) auxiliary_details: AuxiliaryDetails,
    pub(crate) websockets: WebSockets,
    pub(crate) network_requester: Option<NetworkRequesterDetails>,
    pub(crate) ipr: Option<IpPacketRouterDetails>,
    pub(crate) authenticator: Option<AuthenticatorDetails>,
    pub(crate) wireguard: Option<WireguardDetails>,
}

impl UnwrappedResolvedNodeDescribedInfo {
    pub(crate) fn into_node_description(
        self,
        host_info: impl Into<HostInformation>,
    ) -> NymNodeData {
        NymNodeData {
            host_information: host_info.into(),
            last_polled: OffsetDateTime::now_utc().into(),
            build_information: self.build_info,
            network_requester: self.network_requester,
            ip_packet_router: self.ipr,
            authenticator: self.authenticator,
            wireguard: self.wireguard,
            mixnet_websockets: self.websockets,
            auxiliary_details: self.auxiliary_details,
            declared_role: self.roles,
        }
    }
}
