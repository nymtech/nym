// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::api::api_requests;
use crate::node::http::state::AppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::{v1::node::models::SignedHostInformation, SignedDataHostInfo};

/// Returns host information of this node.
#[utoipa::path(
    get,
    path = "/host-information",
    context_path = "/api/v1",
    tag = "Node",
    responses(
        (status = 200, content(
            (SignedDataHostInfo = "application/json"),
            (SignedDataHostInfo = "application/yaml")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn host_information(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> HostInformationResponse {
    let output = output.output.unwrap_or_default();

    let primary_key = state.x25519_sphinx_keys.primary();
    let pre_announced = match state.x25519_sphinx_keys.secondary() {
        None => None,
        Some(secondary_key) => {
            if secondary_key.rotation_id() == primary_key.rotation_id() + 1 {
                Some(api_requests::v1::node::models::SphinxKey {
                    rotation_id: secondary_key.rotation_id(),
                    public_key: secondary_key.x25519_pubkey(),
                })
            } else {
                None
            }
        }
    };

    let primary_pubkey = primary_key.x25519_pubkey();

    #[allow(deprecated)]
    let host_info = api_requests::v1::node::models::HostInformation {
        ip_address: state.static_information.ip_addresses.clone(),
        hostname: state.static_information.hostname.clone(),
        keys: api_requests::v1::node::models::HostKeys {
            ed25519_identity: *state.static_information.ed25519_identity_keys.public_key(),
            x25519_sphinx: primary_pubkey,
            primary_x25519_sphinx_key: api_requests::v1::node::models::SphinxKey {
                rotation_id: primary_key.rotation_id(),
                public_key: primary_pubkey,
            },
            x25519_noise: state.static_information.x25519_noise_key,
            pre_announced_x25519_sphinx_key: pre_announced,
        },
    };

    // SAFETY: the only way for this call to fail is if serialisation of HostInformation fails.
    // however, that conversion is stable and infallible
    #[allow(clippy::unwrap_used)]
    let signed_info = SignedHostInformation::new(
        host_info,
        state.static_information.ed25519_identity_keys.private_key(),
    )
    .unwrap();

    output.to_response(signed_info)
}

pub type HostInformationResponse = FormattedResponse<SignedHostInformation>;
