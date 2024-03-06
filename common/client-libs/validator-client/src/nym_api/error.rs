// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_api_requests::models::RequestError;
use nym_http_api_client::HttpClientError;

pub type NymAPIError = HttpClientError<RequestError>;
