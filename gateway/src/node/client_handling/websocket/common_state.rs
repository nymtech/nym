// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::websocket::connection_handler::ecash::EcashManager;
use crate::node::client_handling::websocket::connection_handler::BandwidthFlushingBehaviourConfig;
use nym_crypto::asymmetric::identity;
use std::sync::Arc;

// I can see this being possible expanded with say storage or client store
#[derive(Clone)]
pub(crate) struct CommonHandlerState {
    pub(crate) ecash_verifier: Arc<EcashManager>,
    pub(crate) local_identity: Arc<identity::KeyPair>,
    pub(crate) only_coconut_credentials: bool,
    pub(crate) offline_credential_verification: bool,
    pub(crate) bandwidth_cfg: BandwidthFlushingBehaviourConfig,
}
