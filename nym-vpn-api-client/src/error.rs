// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::error::Error;

pub use nym_http_api_client::HttpClientError;

use nym_contracts_common::ContractsCommonError;

use crate::response::{ErrorMessage, NymErrorResponse, UnexpectedError};

#[derive(Debug, thiserror::Error)]
pub enum VpnApiClientError {
    #[error("failed tp create vpn api client")]
    CreateVpnApiClient(#[source] HttpClientError<UnexpectedError>),

    #[error("failed to get account")]
    GetAccount(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to get account summary")]
    GetAccountSummary(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to get devices")]
    GetDevices(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to register device")]
    RegisterDevice(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to get active devices")]
    GetActiveDevices(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to get device by id")]
    GetDeviceById(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to get device zk-nym")]
    GetDeviceZkNyms(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to update device")]
    UpdateDevice(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to request zk-nym")]
    RequestZkNym(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to get active zk-nym")]
    GetActiveZkNym(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to get zk-nym by id")]
    GetZkNymById(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to confirm zk-nym download")]
    ConfirmZkNymDownloadById(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to get free passes")]
    GetFreePasses(#[source] HttpClientError<ErrorMessage>),

    #[error("failed to apply free pass")]
    ApplyFreepass(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to get subscriptions")]
    GetSubscriptions(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to create subscription")]
    CreateSubscription(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to get active subscription")]
    GetActiveSubscriptions(#[source] HttpClientError<NymErrorResponse>),

    #[error("failed to get gateways")]
    GetGateways(#[source] HttpClientError<UnexpectedError>),

    #[error("failed to get gateway countries")]
    GetGatewayCountries(#[source] HttpClientError<UnexpectedError>),

    #[error("failed to get entry gateways")]
    GetEntryGateways(#[source] HttpClientError<UnexpectedError>),

    #[error("failed to get entry gateway countries")]
    GetEntryGatewayCountries(#[source] HttpClientError<UnexpectedError>),

    #[error("failed to get exit gateways")]
    GetExitGateways(#[source] HttpClientError<UnexpectedError>),

    #[error("failed to get exit gateway countries")]
    GetExitGatewayCountries(#[source] HttpClientError<UnexpectedError>),

    #[error("failed to get vpn gateways")]
    GetVpnGateways(#[source] HttpClientError<UnexpectedError>),

    #[error("failed to get vpn gateway countries")]
    GetVpnGatewayCountries(#[source] HttpClientError<UnexpectedError>),

    #[error("invalud percent value")]
    InvalidPercentValue(#[source] ContractsCommonError),

    #[error("failed to derive from path")]
    CosmosDeriveFromPath(
        #[source] nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWalletError,
    ),

    #[error("failed to get directory zk-nym ticketbook partial verification keys")]
    GetDirectoryZkNymsTicketbookPartialVerificationKeys(#[source] HttpClientError<ErrorMessage>),

    #[error("failed to get health")]
    GetHealth(#[source] HttpClientError<UnexpectedError>),

    #[error("failed to get usage")]
    GetUsage(#[source] HttpClientError<UnexpectedError>),

    #[error("failed to get registered network environments")]
    GetNetworkEnvs(#[source] HttpClientError<UnexpectedError>),

    #[error("failed to get discovery info")]
    GetDiscoveryInfo(#[source] HttpClientError<UnexpectedError>),

    #[error("failed to get vpn network Details")]
    GetVpnNetworkDetails(#[source] HttpClientError<UnexpectedError>),

    #[error("failed to post account")]
    PostAccount(#[source] HttpClientError<UnexpectedError>),

    #[error("create account")]
    CreateAccount(#[source] crate::types::AccountError),
}

pub type Result<T> = std::result::Result<T, VpnApiClientError>;

impl TryFrom<VpnApiClientError> for NymErrorResponse {
    type Error = VpnApiClientError;

    fn try_from(response: VpnApiClientError) -> std::result::Result<Self, Self::Error> {
        crate::response::extract_error_response(&response).ok_or(response)
    }
}

impl VpnApiClientError {
    pub fn http_client_error<T>(&self) -> Option<&HttpClientError<T>>
    where
        T: std::fmt::Display + std::fmt::Debug + 'static,
    {
        self.source()
            .and_then(|source| source.downcast_ref::<HttpClientError<T>>())
    }
}
