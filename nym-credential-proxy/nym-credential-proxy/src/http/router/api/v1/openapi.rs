// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::router::api;
use crate::http::types::RequestError;
use axum::Router;
use nym_credential_proxy_requests::api as api_requests;
use nym_credential_proxy_requests::routes::api::{v1, v1_absolute};
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

/*
#[derive(OpenApi)]
#[openapi(
    info(title = "Nym VPN Api"),
    paths(
        api::v1::freepass::generate_freepass,
        api::v1::bandwidth_voucher::obtain_bandwidth_voucher_shares,
        api::v1::bandwidth_voucher::obtain_async_bandwidth_voucher_shares,
        api::v1::bandwidth_voucher::current_deposit,
        api::v1::bandwidth_voucher::prehashed_public_attributes,
        api::v1::bandwidth_voucher::partial_verification_keys,
        api::v1::bandwidth_voucher::master_verification_key,
        api::v1::bandwidth_voucher::current_epoch,
        api::v1::bandwidth_voucher::shares::query_for_shares_by_id,
    ),
    components(
        schemas(
            api::Output,
            api::OutputParams,
            api_requests::v1::ErrorResponse,
            api_requests::v1::freepass::models::FreepassCredentialResponse,
            api_requests::v1::freepass::models::FreepassQueryParams,
            api_requests::v1::bandwidth_voucher::models::DepositResponse,
            api_requests::v1::bandwidth_voucher::models::AttributesResponse,
            api_requests::v1::bandwidth_voucher::models::BandwidthVoucherResponse,
            api_requests::v1::bandwidth_voucher::models::BandwidthVoucherAsyncResponse,
            api_requests::v1::bandwidth_voucher::models::PartialVerificationKeysResponse,
            api_requests::v1::bandwidth_voucher::models::CurrentEpochResponse,
            api_requests::v1::bandwidth_voucher::models::CredentialShare,
            api_requests::v1::bandwidth_voucher::models::PartialVerificationKey,
            api_requests::v1::bandwidth_voucher::models::MasterVerificationKeyResponse,
            api_requests::v1::bandwidth_voucher::models::BandwidthVoucherAsyncRequest,
            api_requests::v1::bandwidth_voucher::models::BandwidthVoucherRequest,
            api_requests::v1::bandwidth_voucher::models::BlindSignRequestJsonSchemaWrapper
        ),
        responses(RequestError),
    ),
    modifiers(&SecurityAddon),
)]
pub(crate) struct ApiDoc;
 */

#[derive(OpenApi)]
#[openapi(
    info(title = "Nym Credential Proxy Api"),
    paths(
        api::v1::ticketbook::obtain_ticketbook_shares,
        api::v1::ticketbook::obtain_ticketbook_shares_async,
        api::v1::ticketbook::current_deposit,
        api::v1::ticketbook::partial_verification_keys,
        api::v1::ticketbook::master_verification_key,
        api::v1::ticketbook::current_epoch,
        api::v1::ticketbook::shares::query_for_shares_by_id,
        api::v1::ticketbook::shares::query_for_shares_by_device_id_and_credential_id,
    ),
    components(
        schemas(
            api::Output,
            api::OutputParams,
            api_requests::v1::ErrorResponse,
            api_requests::v1::ticketbook::models::DepositResponse,
            api_requests::v1::ticketbook::models::PartialVerificationKeysResponse,
            api_requests::v1::ticketbook::models::CurrentEpochResponse,
            api_requests::v1::ticketbook::models::PartialVerificationKey,
            api_requests::v1::ticketbook::models::MasterVerificationKeyResponse,
            api_requests::v1::ticketbook::models::TicketbookRequest,
            api_requests::v1::ticketbook::models::TicketbookAsyncRequest,
            api_requests::v1::ticketbook::models::WithdrawalRequestBs58Wrapper,
            api_requests::v1::ticketbook::models::PartialVerificationKey,
            api_requests::v1::ticketbook::models::AggregatedExpirationDateSignaturesResponse,
            api_requests::v1::ticketbook::models::AggregatedCoinIndicesSignaturesResponse,
            api_requests::v1::ticketbook::models::WalletShare,
            api_requests::v1::ticketbook::models::TicketbookWalletSharesResponse,
            api_requests::v1::ticketbook::models::TicketbookWalletSharesAsyncResponse,
            api_requests::v1::ticketbook::models::BlindedWalletSharesResponse,
            api_requests::v1::ticketbook::models::WebhookBlindedSharesResponse,
            api_requests::v1::ticketbook::models::TicketbookObtainQueryParams,
            api_requests::v1::ticketbook::models::SharesQueryParams,
            api_requests::v1::ticketbook::models::PlaceholderJsonSchemaImpl,
        ),
        responses(RequestError),
    ),
    modifiers(&SecurityAddon),
)]
pub(crate) struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "auth_token",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            )
        }
    }
}

// if reverse proxy doesn't work, we might have to look into: https://github.com/juhaku/utoipa/issues/842
pub(crate) fn route<S: Send + Sync + 'static + Clone>() -> Router<S> {
    // provide absolute path to the openapi.json
    let config =
        utoipa_swagger_ui::Config::from(format!("{}/api-docs/openapi.json", v1_absolute()));
    SwaggerUi::new(v1::SWAGGER)
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .config(config)
        .into()
}
