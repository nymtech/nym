// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use http_api_client::HttpClientError;
use nym_api_requests::models::RequestError;

pub type NymAPIError = HttpClientError<RequestError>;
