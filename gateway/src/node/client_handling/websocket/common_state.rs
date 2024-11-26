// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credential_verification::{ecash::EcashManager, BandwidthFlushingBehaviourConfig};
use nym_crypto::asymmetric::identity;
use nym_statistics_common::events::StatsEventSender;
use std::sync::Arc;

// I can see this being possible expanded with say storage or client store
#[derive(Clone)]
pub(crate) struct CommonHandlerState<S> {
    pub(crate) ecash_verifier: Arc<EcashManager<S>>,
    pub(crate) storage: S,
    pub(crate) local_identity: Arc<identity::KeyPair>,
    pub(crate) only_coconut_credentials: bool,
    pub(crate) bandwidth_cfg: BandwidthFlushingBehaviourConfig,
    pub(crate) stats_event_sender: StatsEventSender,
}
