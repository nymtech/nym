// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! An incomplete Zulip API Client
//!
//! Currently, it serves a single purpose: to send a message to a server,
//! however, it could very easily be extended with additional functionalities.
//!
//! ## Sending Direct Message
//!
//! ```rust
//! # use zulip_client::{Client, ZulipClientError};
//! # use zulip_client::message::DirectMessage;
//! # async fn try_send() -> Result<(), ZulipClientError> {
//!   let api_key = "your-api-key";
//!   let email = "associated-email-address";
//!   let server = "https://server-address.com";
//!   let client = Client::builder(email, api_key, server)?.build()?;
//!   // send to userid 12
//!   client.send_message((12u32, "hello world")).await?;
//!   // more concrete typing
//!   client.send_message(DirectMessage::new(12, "hello world2")).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::ZulipClientError;
use crate::message::{DirectMessage, SendMessageResponse, SendableMessage, StreamMessage};
use nym_bin_common::bin_info;
use nym_http_api_client::UserAgent;
use reqwest::{header, Method, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::trace;
use url::Url;
use zeroize::Zeroizing;

#[derive(Serialize, Deserialize)]
pub struct ClientConfig {
    pub user_email: String,
    pub api_key: String,
    // TODO: introduce validation
    pub user_agent: Option<String>,
    pub server_url: Url,
}

pub struct Client {
    server_url: Url,

    api_key: Zeroizing<String>,
    user_email: String,

    inner_client: reqwest::Client,
}

fn default_user_agent() -> String {
    UserAgent::from(bin_info!()).to_string()
}

impl Client {
    const MESSAGES_ENDPOINT: &'static str = "/api/v1/messages";

    pub fn builder(
        user_email: impl Into<String>,
        api_key: impl Into<String>,
        server_url: impl Into<String>,
    ) -> Result<ClientBuilder, ZulipClientError> {
        ClientBuilder::new(user_email, api_key, server_url)
    }

    pub fn new(config: ClientConfig) -> Result<Self, ZulipClientError> {
        let builder = ClientBuilder::new(config.user_email, config.api_key, config.server_url)?;
        match config.user_agent {
            Some(user_agent) => builder.user_agent(user_agent).build(),
            None => builder.build(),
        }
    }

    pub async fn send_message(
        &self,
        msg: impl Into<SendableMessage>,
    ) -> Result<SendMessageResponse, ZulipClientError> {
        let url = format!("{}{}", self.server_url, Self::MESSAGES_ENDPOINT);

        self.build_request(Method::POST, Self::MESSAGES_ENDPOINT)
            .form(&msg.into())
            .send()
            .await
            .map_err(|source| ZulipClientError::RequestSendingFailure { source, url })?
            .json()
            .await
            .map_err(|source| ZulipClientError::RequestDecodeFailure { source })
    }

    pub async fn send_direct_message(
        &self,
        msg: impl Into<DirectMessage>,
    ) -> Result<SendMessageResponse, ZulipClientError> {
        self.send_message(msg.into()).await
    }

    pub async fn send_channel_message(
        &self,
        msg: impl Into<StreamMessage>,
    ) -> Result<SendMessageResponse, ZulipClientError> {
        self.send_message(msg.into()).await
    }

    fn build_request(&self, method: Method, endpoint: &'static str) -> RequestBuilder {
        let url = format!("{}{endpoint}", self.server_url);
        trace!("posting to {url}");

        self.inner_client
            .request(method, url)
            .basic_auth(&self.user_email, Some(self.api_key.to_string()))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
    }
}

pub struct ClientBuilder {
    api_key: Zeroizing<String>,
    user_email: String,
    server_url: Url,
    user_agent: Option<String>,
}

impl ClientBuilder {
    pub fn new(
        user_email: impl Into<String>,
        api_key: impl Into<String>,
        server_url: impl Into<String>,
    ) -> Result<Self, ZulipClientError> {
        let server_url = server_url.into();
        let parsed_url =
            Url::from_str(&server_url).map_err(|source| ZulipClientError::MalformedServerUrl {
                raw: server_url,
                source,
            })?;
        Ok(ClientBuilder {
            api_key: Zeroizing::new(api_key.into()),
            user_email: user_email.into(),
            server_url: parsed_url,
            user_agent: None,
        })
    }

    #[must_use]
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    pub fn build(self) -> Result<Client, ZulipClientError> {
        let user_agent = self.user_agent.unwrap_or_else(default_user_agent);
        Ok(Client {
            api_key: self.api_key,
            server_url: self.server_url,
            user_email: self.user_email,
            inner_client: reqwest::ClientBuilder::new()
                .user_agent(user_agent)
                .build()
                .map_err(|source| ZulipClientError::ClientBuildFailure { source })?,
        })
    }
}
