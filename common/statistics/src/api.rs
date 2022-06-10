// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::StatsError;
use crate::StatsMessage;

pub const DEFAULT_STATISTICS_SERVICE_ADDRESS: &str = "127.0.0.1";
pub const DEFAULT_STATISTICS_SERVICE_PORT: u16 = 8090;

pub const STATISTICS_SERVICE_VERSION: &str = "/v1";
pub const STATISTICS_SERVICE_API_STATISTICS: &str = "statistic";

pub fn build_statistics_request_bytes(msg: StatsMessage) -> Result<Vec<u8>, StatsError> {
    let json_msg = msg.to_json()?;

    let req = reqwest::Request::new(
        reqwest::Method::POST,
        reqwest::Url::parse(&format!(
            "http://{}:{}/{}/{}",
            DEFAULT_STATISTICS_SERVICE_ADDRESS,
            DEFAULT_STATISTICS_SERVICE_PORT,
            STATISTICS_SERVICE_VERSION,
            STATISTICS_SERVICE_API_STATISTICS
        ))
        .unwrap(),
    );
    let data = format!(
        "{} {} {:?}\n\
        Content-Type: application/json\n\
        Content-Length: {}\n\n\
        {}\n",
        req.method().as_str(),
        req.url().as_str(),
        req.version(),
        json_msg.len(),
        json_msg
    );

    Ok(data.into_bytes())
}
