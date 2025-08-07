// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use utoipa::OpenApi;

use crate::models::{Request, Response};

#[derive(OpenApi)]
#[openapi(
    info(title = "Nym Wireguard Private Metadata"),
    tags(),
    components(schemas(Request, Response))
)]
pub(crate) struct ApiDoc;
