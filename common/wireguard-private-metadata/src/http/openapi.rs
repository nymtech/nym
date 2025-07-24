// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use utoipa::OpenApi;

use crate::models::{AvailableBandwidthResponse, TopUpRequest};

#[derive(OpenApi)]
#[openapi(
    info(title = "Nym Wireguard Private Metadata"),
    tags(),
    components(schemas(AvailableBandwidthResponse, TopUpRequest))
)]
pub(crate) struct ApiDoc;
