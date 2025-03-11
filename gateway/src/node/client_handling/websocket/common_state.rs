// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::ActiveClientsStore;
use nym_credential_verification::{ecash::EcashManager, BandwidthFlushingBehaviourConfig};
use nym_crypto::asymmetric::identity;
use nym_gateway_storage::GatewayStorage;
use nym_mixnet_client::forwarder::MixForwardingSender;
use nym_node_metrics::events::MetricEventsSender;
use nym_node_metrics::NymNodeMetrics;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct Config {
    pub(crate) enforce_zk_nym: bool,
    pub(crate) max_auth_request_age: Duration,

    pub(crate) bandwidth: BandwidthFlushingBehaviourConfig,
}

// I can see this being possible expanded with say storage or client store
#[derive(Clone)]
pub(crate) struct CommonHandlerState {
    pub(crate) cfg: Config,
    pub(crate) ecash_verifier: Arc<EcashManager>,
    pub(crate) storage: GatewayStorage,
    pub(crate) local_identity: Arc<identity::KeyPair>,
    pub(crate) metrics: NymNodeMetrics,
    pub(crate) metrics_sender: MetricEventsSender,
    pub(crate) outbound_mix_sender: MixForwardingSender,
    pub(crate) active_clients_store: ActiveClientsStore,
}

impl CommonHandlerState {
    pub(crate) fn storage(&self) -> &GatewayStorage {
        &self.storage
    }
}
