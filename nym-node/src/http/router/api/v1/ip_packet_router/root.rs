// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::router::api::{FormattedResponse, OutputParams};
use axum::extract::Query;
use axum::http::StatusCode;
use nym_node_requests::api::v1::ip_packet_router::models::IpPacketRouter;

/// Returns root network requester information
#[utoipa::path(
    get,
    path = "",
    context_path = "/api/v1/ip-packet-router",
    tag = "IP Packet Router",
    responses(
        (status = 501, description = "the endpoint hasn't been implemented yet"),
        (status = 200, content(
            ("application/json" = IpPacketRouter),
            ("application/yaml" = IpPacketRouter)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn root_ip_packet_router(
    details: Option<IpPacketRouter>,
    Query(output): Query<OutputParams>,
) -> Result<IpPacketRouterResponse, StatusCode> {
    let details = details.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(details))
}

pub type IpPacketRouterResponse = FormattedResponse<IpPacketRouter>;
