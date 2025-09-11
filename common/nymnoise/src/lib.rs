// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_noise_keys::NoiseVersion;
use snow::error::Prerequisite;
use snow::Error;
use tokio::net::TcpStream;
use tracing::{error, warn};

pub mod config;
pub mod connection;
pub mod error;
pub mod stream;

use crate::config::NoiseConfig;
use crate::connection::Connection;
use crate::error::NoiseError;
use crate::stream::NoiseStreamBuilder;

const NOISE_PSK_PREFIX: &[u8] = b"NYMTECH_NOISE_dQw4w9WgXcQ";

pub const LATEST_NOISE_VERSION: NoiseVersion = NoiseVersion::V1;

// TODO: this should be behind some trait because presumably, depending on the version,
// other arguments would be needed
mod psk_gen {
    use crate::error::NoiseError;
    use crate::stream::Psk;
    use crate::NOISE_PSK_PREFIX;
    use nym_crypto::asymmetric::x25519;
    use nym_noise_keys::NoiseVersion;
    use sha2::{Digest, Sha256};

    pub(crate) fn generate_psk(
        responder_pub_key: x25519::PublicKey,
        version: NoiseVersion,
    ) -> Result<Psk, NoiseError> {
        match version {
            NoiseVersion::V1 => Ok(generate_psk_v1(responder_pub_key)),
            NoiseVersion::Unknown(noise_version) => {
                Err(NoiseError::PskGenerationFailure { noise_version })
            }
        }
    }

    fn generate_psk_v1(responder_pub_key: x25519::PublicKey) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(NOISE_PSK_PREFIX);
        hasher.update(responder_pub_key.to_bytes());
        hasher.finalize().into()
    }
}

pub async fn upgrade_noise_initiator(
    conn: TcpStream,
    config: &NoiseConfig,
) -> Result<Connection<TcpStream>, NoiseError> {
    if config.unsafe_disabled {
        warn!("Noise is disabled in the config. Not attempting any handshake");
        return Ok(Connection::Raw(conn));
    }

    //Get init material
    let responder_addr = conn.peer_addr().map_err(|err| {
        error!("Unable to extract peer address from connection - {err}");
        Error::Prereq(Prerequisite::RemotePublicKey)
    })?;

    let Some(key) = config.get_noise_key(&responder_addr) else {
        warn!("{responder_addr} can't speak Noise yet, falling back to TCP");
        return Ok(Connection::Raw(conn));
    };

    let handshake_version = match key.supported_version {
        NoiseVersion::V1 => NoiseVersion::V1,

        // We're talking to a more recent node, but we can't adapt. Let's try to do our best and if it fails, it fails.
        // If that node sees we're older, it will try to adapt too.
        NoiseVersion::Unknown(version) => {
            warn!("{responder_addr} is announcing an v{version} version of Noise that we don't know how to parse, we will attempt to downgrade to our current highest supported version");
            LATEST_NOISE_VERSION
        }
    };

    NoiseStreamBuilder::new(conn)
        .perform_initiator_handshake(config, handshake_version, key.x25519_pubkey)
        .await
        .map(|stream| Connection::Noise(Box::new(stream)))
}
pub async fn upgrade_noise_responder(
    conn: TcpStream,
    config: &NoiseConfig,
) -> Result<Connection<TcpStream>, NoiseError> {
    if config.unsafe_disabled {
        warn!("Noise is disabled in the config. Not attempting any handshake");
        return Ok(Connection::Raw(conn));
    }

    //Get init material
    let initiator_addr = match conn.peer_addr() {
        Ok(addr) => addr,
        Err(err) => {
            error!("Unable to extract peer address from connection - {err}");
            return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
        }
    };

    // if responder doesn't announce noise support, we fallback to tcp
    if config.get_noise_support(initiator_addr.ip()).is_none() {
        warn!("{initiator_addr} can't speak Noise yet, falling back to TCP",);
        return Ok(Connection::Raw(conn));
    };

    NoiseStreamBuilder::new(conn)
        .perform_responder_handshake(config)
        .await
        .map(|stream| Connection::Noise(Box::new(stream)))
}
