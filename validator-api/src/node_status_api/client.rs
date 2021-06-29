// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{BatchGatewayStatus, BatchMixStatus, DefaultRestResponse};
use crate::node_status_api::NodeStatusApiClientError;

pub(crate) struct Config {
    base_url: String,
}

impl Config {
    pub(crate) fn new<S: Into<String>>(base_url: S) -> Self {
        Config {
            base_url: base_url.into(),
        }
    }
}

pub(crate) struct Client {
    config: Config,
    reqwest_client: reqwest::Client,
}

impl Client {
    pub(crate) fn new(config: Config) -> Self {
        let reqwest_client = reqwest::Client::new();
        Client {
            config,
            reqwest_client,
        }
    }

    // Potentially, down the line, this could be moved to /common/client-libs
    // and additional methods could be added like GET for report data, but currently
    // we have absolutely no use for that in Rust.

    pub(crate) async fn post_batch_mix_status(
        &self,
        batch_status: BatchMixStatus,
    ) -> Result<(), NodeStatusApiClientError> {
        const RELATIVE_PATH: &str = "api/status/mixnode/batch";

        let url = format!("{}/{}", self.config.base_url, RELATIVE_PATH);

        let response = self
            .reqwest_client
            .post(url)
            .json(&batch_status)
            .send()
            .await?;

        if response.status().is_success() {
            let response_content: DefaultRestResponse = response.json().await?;
            match response_content {
                DefaultRestResponse::Ok(ok_response) => {
                    if ok_response.ok {
                        Ok(())
                    } else {
                        Err(NodeStatusApiClientError::NodeStatusApiError(
                            "received an ok response with false status".into(),
                        ))
                    }
                }
                DefaultRestResponse::Error(err_response) => Err(err_response.into()),
            }
        } else {
            Err(NodeStatusApiClientError::NodeStatusApiError(format!(
                "received response with status {}",
                response.status()
            )))
        }
    }

    pub(crate) async fn post_batch_gateway_status(
        &self,
        batch_status: BatchGatewayStatus,
    ) -> Result<(), NodeStatusApiClientError> {
        const RELATIVE_PATH: &str = "api/status/gateway/batch";

        let url = format!("{}/{}", self.config.base_url, RELATIVE_PATH);

        let response = self
            .reqwest_client
            .post(url)
            .json(&batch_status)
            .send()
            .await?;

        if response.status().is_success() {
            let response_content: DefaultRestResponse = response.json().await?;
            match response_content {
                DefaultRestResponse::Ok(ok_response) => {
                    if ok_response.ok {
                        Ok(())
                    } else {
                        Err(NodeStatusApiClientError::NodeStatusApiError(
                            "received an ok response with false status".into(),
                        ))
                    }
                }
                DefaultRestResponse::Error(err_response) => Err(err_response.into()),
            }
        } else {
            Err(NodeStatusApiClientError::NodeStatusApiError(format!(
                "received response with status {}",
                response.status()
            )))
        }
    }
}
