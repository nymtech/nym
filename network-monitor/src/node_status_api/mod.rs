// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::ErrorResponses;
use std::fmt::{self, Display, Formatter};

mod client;
pub(crate) mod models;

pub(crate) use client::{Client, Config};

const MAX_SANE_UNEXPECTED_PRINT: usize = 100;

#[derive(Debug)]
pub enum NodeStatusApiClientError {
    ReqwestClientError(reqwest::Error),
    NodeStatusApiError(String),
    UnexpectedResponse(String),
}

impl From<reqwest::Error> for NodeStatusApiClientError {
    fn from(err: reqwest::Error) -> Self {
        NodeStatusApiClientError::ReqwestClientError(err)
    }
}

impl From<ErrorResponses> for NodeStatusApiClientError {
    fn from(err: ErrorResponses) -> Self {
        match err {
            ErrorResponses::Error(err_message) => {
                NodeStatusApiClientError::NodeStatusApiError(err_message.error)
            }
            ErrorResponses::Unexpected(received) => {
                NodeStatusApiClientError::UnexpectedResponse(received.to_string())
            }
        }
    }
}

impl Display for NodeStatusApiClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            NodeStatusApiClientError::ReqwestClientError(err) => {
                write!(f, "there was an issue with the REST request - {}", err)
            }
            NodeStatusApiClientError::NodeStatusApiError(err) => {
                write!(
                    f,
                    "there was an issue with the node status api client - {}",
                    err
                )
            }
            NodeStatusApiClientError::UnexpectedResponse(received) => {
                if received.len() < MAX_SANE_UNEXPECTED_PRINT {
                    write!(
                        f,
                        "received data was completely unexpected. got: {}",
                        received
                    )
                } else {
                    write!(
                        f,
                        "received data was completely unexpected. got: {}...",
                        received
                            .chars()
                            .take(MAX_SANE_UNEXPECTED_PRINT)
                            .collect::<String>()
                    )
                }
            }
        }
    }
}
