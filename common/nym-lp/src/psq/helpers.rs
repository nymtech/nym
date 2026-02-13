// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::codec::{OuterAeadKey, parse_lp_packet, serialize_lp_packet};
use crate::{LpError, LpPacket};
use bytes::BytesMut;
use nym_lp_transport::traits::LpTransport;

use libcrux_psq::handshake::ciphersuites::CiphersuiteName;
#[cfg(test)]
use mock_instant::thread_local::{SystemTime, UNIX_EPOCH};
use nym_kkt_ciphersuite::KEM;
#[cfg(not(test))]
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn kem_to_ciphersuite(kem: KEM) -> CiphersuiteName {
    match kem {
        KEM::MlKem768 => CiphersuiteName::X25519_MLKEM768_X25519_AESGCM128_HKDFSHA256,
        KEM::McEliece => CiphersuiteName::X25519_CLASSICMCELIECE_X25519_AESGCM128_HKDFSHA256,
    }
}

pub(crate) fn current_timestamp() -> Result<u64, LpError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| LpError::Internal("System time before UNIX epoch".into()))
        .map(|d| d.as_secs())
}

// only used in internal code (and tests)
#[allow(async_fn_in_trait)]
pub trait LpTransportHandshakeExt: LpTransport {
    // the outer key is temporary until the algorithm is changed with psqv2
    async fn receive_packet(
        &mut self,
        outer_key: Option<&OuterAeadKey>,
    ) -> Result<LpPacket, LpError>
    where
        Self: Unpin,
    {
        let raw = self.receive_raw_packet().await?;
        parse_lp_packet(&raw, outer_key)
    }

    async fn send_packet(
        &mut self,
        packet: LpPacket,
        outer_key: Option<&OuterAeadKey>,
    ) -> Result<(), LpError>
    where
        Self: Unpin,
    {
        let mut packet_buf = BytesMut::new();

        serialize_lp_packet(&packet, &mut packet_buf, outer_key)?;
        self.send_serialised_packet(&packet_buf).await?;
        Ok(())
    }
}

impl<T> LpTransportHandshakeExt for T where T: LpTransport {}
