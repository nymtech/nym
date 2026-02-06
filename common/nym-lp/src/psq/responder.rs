// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpError;
use crate::psq::{LPSession, PSQHandshakeState};
use nym_lp_transport::traits::LpTransport;

impl<'a, S> PSQHandshakeState<'a, S> {
    pub async fn psq_handshake_responder(self) -> Result<LPSession, LpError>
    where
        S: LpTransport + Unpin,
    {
        // 1. receive ClientHello

        // 2. send ack

        // 3. receive KKT request

        // 4. send KKT response

        // 5. receive PSQ msg1

        // 6. send PSQ msg2

        // 7. receive PSQ msg3

        // 8. send ACK
        todo!()
    }
}
