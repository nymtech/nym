// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::error::LpHandlerError;
use crate::node::lp::state::SharedLpControlState;
use nym_lp::transport::LpHandshakeChannel;
use nym_lp::{KEM, KEMKeyDigests, LpTransportSession};
use std::collections::BTreeMap;

/// Complete the KKT/PSQ handshake on an accepted control connection, within the configured TTL.
pub(crate) async fn complete_ingress_handshake<S>(
    stream: &mut S,
    state: &SharedLpControlState,
    initiator_kem_hashes: Option<BTreeMap<KEM, KEMKeyDigests>>,
) -> Result<LpTransportSession, LpHandlerError>
where
    S: LpHandshakeChannel + Unpin,
{
    let timeout = state.lp_config.debug.handshake_ttl;
    let local_peer = state.local_lp_peer.clone();

    match tokio::time::timeout(timeout, async move {
        LpTransportSession::psq_handshake_responder(stream, local_peer, initiator_kem_hashes)
            .complete_handshake()
            .await
    })
    .await
    {
        Err(_timeout) => Err(LpHandlerError::HandshakeTimeout),
        Ok(result) => Ok(result?),
    }
}
