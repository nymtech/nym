// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::router::api;
use axum::Router;
use nym_credential_proxy_requests::api as api_requests;
use nym_credential_proxy_requests::routes::api::{v1, v1_absolute};
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;
use utoipauto::utoipauto;

// `#[utoipa::path]` handlers in this crate are autodiscovered, so the `paths(...)` list no
// longer needs maintaining by hand. The schemas below cannot be: both the
// `nym-credential-proxy-requests` models and `Output`/`OutputParams` derive `ToSchema` behind a
// feature flag with `cfg_attr`, and proc macros run before feature flags so utoipauto can't see
// them. Tracking issue: https://github.com/ProbablyClem/utoipauto/issues/13
#[utoipauto(paths = "./nym-credential-proxy/nym-credential-proxy/src")]
#[derive(OpenApi)]
#[openapi(
    info(title = "Nym Credential Proxy Api"),
    modifiers(&SecurityAddon),
    components(schemas(
        api::Output,
        api::OutputParams,
        api_requests::v1::ticketbook::models::DepositResponse,
        api_requests::v1::ticketbook::models::PartialVerificationKeysResponse,
        api_requests::v1::ticketbook::models::CurrentEpochResponse,
        api_requests::v1::ticketbook::models::PartialVerificationKey,
        api_requests::v1::ticketbook::models::MasterVerificationKeyResponse,
        api_requests::v1::ticketbook::models::TicketbookRequest,
        api_requests::v1::ticketbook::models::TicketbookAsyncRequest,
        api_requests::v1::ticketbook::models::WithdrawalRequestBs58Wrapper,
        api_requests::v1::ticketbook::models::AggregatedExpirationDateSignaturesResponse,
        api_requests::v1::ticketbook::models::AggregatedCoinIndicesSignaturesResponse,
        api_requests::v1::ticketbook::models::WalletShare,
        api_requests::v1::ticketbook::models::TicketbookWalletSharesResponse,
        api_requests::v1::ticketbook::models::TicketbookWalletSharesAsyncResponse,
        api_requests::v1::ticketbook::models::WebhookTicketbookWalletShares,
        api_requests::v1::ticketbook::models::WebhookTicketbookWalletSharesRequest,
        api_requests::v1::ticketbook::models::TicketbookObtainQueryParams,
        api_requests::v1::ticketbook::models::SharesQueryParams,
        api_requests::v1::ticketbook::models::PlaceholderJsonSchemaImpl
    ))
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
