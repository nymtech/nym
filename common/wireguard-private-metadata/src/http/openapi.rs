// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use utoipa::OpenApi;

use crate::models::AvailableBandwidth;

#[derive(OpenApi)]
#[openapi(
    info(title = "Nym Wireguard Private Metadata"),
    tags(),
    components(schemas(AvailableBandwidth))
)]
pub(crate) struct ApiDoc;
