// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::NoiseConfig;
use crate::connection::Connection;
use crate::error::NoiseError;
use crate::stream::NoiseStream;
use nym_crypto::asymmetric::x25519;
use nym_noise_keys::NoiseVersion;
use sha2::{Digest, Sha256};
use snow::{error::Prerequisite, Error};
use tokio::net::TcpStream;
use tracing::*;

pub mod config;
pub mod connection;
pub mod error;
pub mod stream;

const NOISE_PSK_PREFIX: &[u8] = b"NYMTECH_NOISE_dQw4w9WgXcQ";

pub const LATEST_NOISE_VERSION: NoiseVersion = NoiseVersion::V1;

fn generate_psk_v1(responder_pub_key: &x25519::PublicKey) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(NOISE_PSK_PREFIX);
    hasher.update(responder_pub_key.to_bytes());
    hasher.finalize().into()
}

async fn upgrade_noise_initiator_v1(
    conn: TcpStream,
    config: &NoiseConfig,
    remote_pub_key: &x25519::PublicKey,
) -> Result<Connection, NoiseError> {
    trace!("Perform Noise Handshake, initiator side");

    let secret_hash = generate_psk_v1(remote_pub_key);
    let noise_stream = NoiseStream::new_initiator(conn, config, remote_pub_key, &secret_hash)?;

    Ok(Connection::Noise(
        tokio::time::timeout(config.timeout, noise_stream.perform_handshake()).await??,
    ))
}

pub async fn upgrade_noise_initiator(
    conn: TcpStream,
    config: &NoiseConfig,
) -> Result<Connection, NoiseError> {
    if config.unsafe_disabled {
        warn!("Noise is disabled in the config. Not attempting any handshake");
        return Ok(Connection::Tcp(conn));
    }

    //Get init material
    let responder_addr = conn.peer_addr().map_err(|err| {
        error!("Unable to extract peer address from connection - {err}");
        Error::Prereq(Prerequisite::RemotePublicKey)
    })?;

    match config.get_noise_key(&responder_addr) {
        Some(key) => match key.version {
            NoiseVersion::V1 => upgrade_noise_initiator_v1(conn, config, &key.x25519_pubkey).await,
            // We're talking to a more recent node, but we can't adapt. Let's try to do our best and if it fails, it fails.
            // If that node sees we're older, it will try to adapt too.
            NoiseVersion::Unknown => {
                warn!("{responder_addr} is announcing an unknown version of Noise, we will still attempt our latest known version");
                upgrade_noise_initiator_v1(conn, config, &key.x25519_pubkey)
                    .await
                    .or(Err(NoiseError::UnknownVersion))
            }
        },
        None => {
            warn!("{responder_addr} can't speak Noise yet, falling back to TCP");
            Ok(Connection::Tcp(conn))
        }
    }
}

async fn upgrade_noise_responder_v1(
    conn: TcpStream,
    config: &NoiseConfig,
) -> Result<Connection, NoiseError> {
    trace!("Perform Noise Handshake, responder side");

    let secret_hash = generate_psk_v1(config.local_key.public_key());
    let noise_stream = NoiseStream::new_responder(conn, config, &secret_hash)?;

    Ok(Connection::Noise(
        tokio::time::timeout(config.timeout, noise_stream.perform_handshake()).await??,
    ))
}

pub async fn upgrade_noise_responder(
    conn: TcpStream,
    config: &NoiseConfig,
) -> Result<Connection, NoiseError> {
    if config.unsafe_disabled {
        warn!("Noise is disabled in the config. Not attempting any handshake");
        return Ok(Connection::Tcp(conn));
    }

    //Get init material
    let initiator_addr = match conn.peer_addr() {
        Ok(addr) => addr,
        Err(err) => {
            error!("Unable to extract peer address from connection - {err}");
            return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
        }
    };

    // Port is random and we just need the support info
    match config.get_noise_support(initiator_addr.ip()) {
        None => {
            warn!("{initiator_addr} can't speak Noise yet, falling back to TCP",);
            Ok(Connection::Tcp(conn))
        }
        // responder's info on version is shaky, so ideally, initiator has to adapt.
        // if we are newer, it won't ba able to, so let's try to meet him on his ground.
        Some(LATEST_NOISE_VERSION) | Some(NoiseVersion::Unknown) => {
            // Node is announcing the same version as us, great or
            // Node is announcing a newer version than us, it should adapt to us though
            upgrade_noise_responder_v1(conn, config).await
        } //SW sample of code to allow backwards compatibility when we introduce new versions
          // Some(IntermediateNoiseVersion) => {
          // Node is announcing an older version, let's try to adapt
          //    upgrade_noise_responder_Vwhatever
          // }
    }
}
