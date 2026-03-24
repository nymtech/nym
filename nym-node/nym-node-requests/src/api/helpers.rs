// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

/// The default HTTP API port exposed by nym-nodes that are not behind a reverse proxy.
pub const STANDARD_NYM_NODE_HTTP_PORT: u16 = 8080;

#[cfg(feature = "client")]
pub use client_helpers::*;

#[cfg(feature = "client")]
mod client_helpers {
    use crate::api::SignedHostInformation;
    use crate::api::client::NymNodeApiClientExt;
    use nym_http_api_client::UserAgent;
    use std::time::Duration;

    use super::STANDARD_NYM_NODE_HTTP_PORT;

    /// Builder-style helper for obtaining a validated [`crate::api::Client`] for a nym-node.
    ///
    /// On top of the basic port-probing performed by [`try_get_valid_nym_node_api_client`],
    /// this struct optionally:
    /// - verifies that the node's self-reported ed25519 identity matches an expected value
    ///   (e.g. the identity committed on-chain during bonding), and
    /// - checks the cryptographic signature on the node's host information.
    ///
    /// Both checks require an extra HTTP round-trip to the node's `/host-information` endpoint
    /// and are skipped when neither option is enabled.
    #[derive(Debug)]
    pub struct NymNodeApiClientRetriever {
        /// Expected (base58-encoded) ed25519 identity of the node.
        /// used to check against data retrieved from the host information
        expected_identity: Option<String>,

        /// Custom port to use when attempting to query the node.
        custom_port: Option<u16>,

        /// User agent to use when attempting to query the node.
        user_agent: UserAgent,

        /// Specify whether the signature on the host information should be verified.
        verify_host_information: bool,
    }

    impl NymNodeApiClientRetriever {
        /// Creates a new retriever with the given user agent.
        /// All optional checks (identity verification, host information signature)
        /// are disabled by default — use the builder methods to enable them.
        pub fn new(user_agent: impl Into<UserAgent>) -> Self {
            Self {
                expected_identity: None,
                custom_port: None,
                user_agent: user_agent.into(),
                verify_host_information: false,
            }
        }

        /// If set, the node's self-reported ed25519 identity (from its `/host-information`
        /// endpoint) will be compared against this value. A mismatch produces
        /// [`crate::error::Error::MismatchedIdentity`].
        #[must_use]
        pub fn with_expected_identity(mut self, expected_identity: Option<String>) -> Self {
            self.expected_identity = expected_identity;
            self
        }

        /// Prepend `http://<host>:<port>` to the list of addresses probed during
        /// [`get_client`](Self::get_client), so it is tried before the standard ports.
        #[must_use]
        pub fn with_custom_port(mut self, port: Option<u16>) -> Self {
            self.custom_port = port;
            self
        }

        /// Enable cryptographic verification of the node's host information signature.
        /// When enabled, [`get_client`](Self::get_client) will return
        /// [`crate::error::Error::MissignedHostInformation`] if the signature is invalid.
        #[must_use]
        pub fn with_verify_host_information(mut self) -> Self {
            self.verify_host_information = true;
            self
        }

        /// Probe the node's HTTP API, perform any configured verification, and return the
        /// client together with the [`SignedHostInformation`] if it was fetched.
        ///
        /// The host information is only retrieved when identity verification or signature
        /// checking is enabled. When neither is active, the returned
        /// [`ApiClientWithHostInformation::host_information`] will be `None`.
        pub async fn get_client(
            self,
            base_host: &str,
            node_id: u32,
        ) -> Result<ApiClientWithHostInformation, crate::error::Error> {
            let base_client = try_get_valid_nym_node_api_client(
                base_host,
                node_id,
                self.custom_port,
                self.user_agent,
            )
            .await?;

            // no need to retrieve host information if we don't have to perform any verification
            if !self.verify_host_information && self.expected_identity.is_none() {
                return Ok(base_client.into());
            }

            let host_info = retrieve_validated_host_information(
                &base_client,
                node_id,
                &self.expected_identity,
                self.verify_host_information,
            )
            .await?;

            Ok(ApiClientWithHostInformation::from(base_client).with_host_information(host_info))
        }
    }

    /// Fetch a node's [`SignedHostInformation`] and optionally validate it.
    ///
    /// This is the standalone equivalent of the checks performed inside
    /// [`NymNodeApiClientRetriever::get_client`], useful when the caller already
    /// holds a [`crate::api::Client`] and only needs the host information.
    ///
    /// When `expected_ed25519_identity` is `Some`, the node's self-reported identity
    /// is compared against it — a mismatch produces [`crate::error::Error::MismatchedIdentity`].
    /// When `verify_host_information` is `true`, the cryptographic signature on the
    /// host information is checked — an invalid signature produces
    /// [`crate::error::Error::MissignedHostInformation`].
    pub async fn retrieve_validated_host_information(
        client: &crate::api::Client,
        node_id: u32,
        expected_ed25519_identity: &Option<String>,
        verify_host_information: bool,
    ) -> Result<SignedHostInformation, crate::error::Error> {
        let host_info = match client.get_host_information().await {
            Ok(info) => info,
            Err(err) => {
                return Err(crate::error::Error::QueryFailure {
                    host: client.current_url().to_string(),
                    node_id,
                    source: Box::new(err),
                });
            }
        };

        if let Some(expected_identity) = expected_ed25519_identity {
            // check if the identity key matches the information provided during bonding
            if expected_identity.as_str() != host_info.keys.ed25519_identity.to_base58_string() {
                return Err(crate::error::Error::MismatchedIdentity {
                    node_id,
                    expected: expected_identity.clone(),
                    got: host_info.keys.ed25519_identity.to_base58_string(),
                });
            }
        }

        // check if the host information has been signed with the node's key
        if verify_host_information && !host_info.verify_host_information() {
            return Err(crate::error::Error::MissignedHostInformation { node_id });
        }

        Ok(host_info)
    }

    /// A nym-node API client bundled with the node's [`SignedHostInformation`],
    /// if it was retrieved during the connection/verification phase.
    ///
    /// This avoids a redundant second call to the `/host-information` endpoint
    /// when the caller also needs the host information after obtaining the client.
    pub struct ApiClientWithHostInformation {
        pub client: crate::api::Client,
        pub host_information: Option<SignedHostInformation>,
    }

    impl ApiClientWithHostInformation {
        fn with_host_information(self, host_information: SignedHostInformation) -> Self {
            Self {
                host_information: Some(host_information),
                ..self
            }
        }
    }

    impl From<crate::api::Client> for ApiClientWithHostInformation {
        fn from(client: crate::api::Client) -> Self {
            Self {
                client,
                host_information: None,
            }
        }
    }

    /// Probe a nym-node's HTTP API and return a connected [`crate::api::Client`].
    ///
    /// `base_host` is a hostname (e.g. `nymtech.net`) or IP address (e.g. `127.0.0.1`).
    /// The function tries the following addresses in order, returning the first one whose
    /// `/health` endpoint reports an "up" status:
    ///
    /// 1. `http://<host>:<custom_port>` (only when `custom_port` is `Some`)
    /// 2. `http://<host>:8080` — the standard nym-node API port
    /// 3. `https://<host>` — node behind an HTTPS reverse proxy (port 443)
    /// 4. `http://<host>` — node behind an HTTP reverse proxy (port 80)
    ///
    /// This function is intended for infrastructure binaries (nym-api, network monitor, etc.),
    /// not regular clients, which is why hickory DNS is explicitly disabled.
    pub async fn try_get_valid_nym_node_api_client(
        base_host: &str,
        node_id: u32,
        custom_port: Option<u16>,
        user_agent: impl Into<UserAgent>,
    ) -> Result<crate::api::Client, crate::error::Error> {
        // first try the standard port in case the operator didn't put the node behind the proxy,
        // then default https (443)
        // finally default http (80)
        let mut addresses_to_try = vec![
            format!("http://{base_host}:{STANDARD_NYM_NODE_HTTP_PORT}"), // 'standard' nym-node
            format!("https://{base_host}"), // node behind https proxy (443)
            format!("http://{base_host}"),  // node behind http proxy (80)
        ];

        // if a custom port was provided, try to connect to it first
        if let Some(port) = custom_port {
            addresses_to_try.insert(0, format!("http://{base_host}:{port}"));
        }

        let user_agent = user_agent.into();
        for address in addresses_to_try {
            // if provided base_host was malformed, there's no point in continuing
            let client = match crate::api::Client::builder(address).and_then(|b| {
                b.with_timeout(Duration::from_secs(5))
                    .no_hickory_dns()
                    .with_user_agent(user_agent.clone())
                    .build()
            }) {
                Ok(client) => client,
                Err(err) => {
                    return Err(crate::error::Error::MalformedHost {
                        host: base_host.to_string(),
                        node_id,
                        source: Box::new(err),
                    });
                }
            };

            if let Ok(health) = client.get_health().await
                && health.status.is_up()
            {
                return Ok(client);
            }
        }

        Err(crate::error::Error::NoHttpPortsAvailable {
            host: base_host.to_string(),
            node_id,
        })
    }
}
