// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::MixnodeError;
use crate::node::node_description::NodeDescription;
use log::info;
use nym_bin_common::bin_info_owned;
use nym_crypto::asymmetric::{encryption, identity};
use nym_node_http_api::api::api_requests;
use nym_node_http_api::api::api_requests::SignedHostInformation;
use nym_node_http_api::state::metrics::{SharedMixingStats, SharedVerlocStats};
use nym_node_http_api::NymNodeHttpError;
use nym_task::TaskClient;

pub(crate) mod legacy;

fn load_host_details(
    config: &Config,
    sphinx_key: &encryption::PublicKey,
    identity_keypair: &identity::KeyPair,
) -> Result<api_requests::v1::node::models::SignedHostInformation, MixnodeError> {
    let host_info = api_requests::v1::node::models::HostInformation {
        ip_address: config.host.public_ips.clone(),
        hostname: config.host.hostname.clone(),
        keys: api_requests::v1::node::models::HostKeys {
            ed25519: identity_keypair.public_key().to_base58_string(),
            x25519: sphinx_key.to_base58_string(),
        },
    };

    let signed_info = SignedHostInformation::new(host_info, identity_keypair.private_key())
        .map_err(NymNodeHttpError::from)?;
    Ok(signed_info)
}

fn load_mixnode_details(
    _config: &Config,
) -> Result<api_requests::v1::mixnode::models::Mixnode, MixnodeError> {
    Ok(api_requests::v1::mixnode::models::Mixnode {})
}

pub(crate) struct HttpApiBuilder<'a> {
    mixnode_config: &'a Config,
    identity_keypair: &'a identity::KeyPair,
    sphinx_keypair: &'a encryption::KeyPair,
    legacy_mixnode: legacy::state::MixnodeAppState,
    legacy_descriptor: NodeDescription,
}

impl<'a> HttpApiBuilder<'a> {
    pub(crate) fn new(
        mixnode_config: &'a Config,
        identity_keypair: &'a identity::KeyPair,
        sphinx_keypair: &'a encryption::KeyPair,
    ) -> Self {
        HttpApiBuilder {
            mixnode_config,
            identity_keypair,
            sphinx_keypair,
            legacy_mixnode: legacy::state::MixnodeAppState::default(),
            legacy_descriptor: Default::default(),
        }
    }

    #[must_use]
    pub(crate) fn with_metrics_key(mut self, metrics_key: Option<&String>) -> Self {
        self.legacy_mixnode.metrics_key = metrics_key.map(|k| k.to_string());
        self
    }

    #[must_use]
    pub(crate) fn with_verloc(mut self, verloc: SharedVerlocStats) -> Self {
        self.legacy_mixnode.verloc = verloc;
        self
    }

    #[must_use]
    pub(crate) fn with_mixing_stats(mut self, stats: SharedMixingStats) -> Self {
        self.legacy_mixnode.stats = stats;
        self
    }

    #[must_use]
    pub(crate) fn with_descriptor(mut self, descriptor: NodeDescription) -> Self {
        self.legacy_descriptor = descriptor;
        self
    }

    pub(crate) fn start(self, task_client: TaskClient) -> Result<(), MixnodeError> {
        let bind_address = self.mixnode_config.http.bind_address;
        info!("Starting HTTP API on http://{bind_address}",);

        let config = nym_node_http_api::Config::new(
            bin_info_owned!(),
            load_host_details(
                self.mixnode_config,
                self.sphinx_keypair.public_key(),
                self.identity_keypair,
            )?,
        )
        .with_mixnode(load_mixnode_details(self.mixnode_config)?)
        .with_landing_page_assets(self.mixnode_config.http.landing_page_assets_path.as_ref());

        let router = nym_node_http_api::NymNodeRouter::new(config, None, None);
        let server = router
            .with_merged(legacy::routes(self.legacy_mixnode, self.legacy_descriptor))
            .build_server(&bind_address)?
            .with_task_client(task_client);
        tokio::spawn(async move { server.run().await });
        Ok(())
    }
}
