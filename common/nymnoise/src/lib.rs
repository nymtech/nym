// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::{NoiseConfig, NoisePattern};
use crate::connection::Connection;
use crate::error::NoiseError;
use crate::stream::NoiseStream;
use nym_crypto::asymmetric::x25519;
use nym_noise_keys::NoiseVersion;
use sha2::{Digest, Sha256};
use snow::{error::Prerequisite, Builder, Error};
use tokio::net::TcpStream;
use tracing::*;

pub mod config;
pub mod connection;
pub mod error;
pub mod stream;

const NOISE_PSK_PREFIX: &[u8] = b"NYMTECH_NOISE_dQw4w9WgXcQ";

pub const NOISE_VERSION: NoiseVersion = NoiseVersion::V1;

async fn upgrade_noise_initiator_v1(
    conn: TcpStream,
    pattern: NoisePattern,
    local_private_key: &x25519::PrivateKey,
    remote_pub_key: &x25519::PublicKey,
) -> Result<Connection, NoiseError> {
    trace!("Perform Noise Handshake, initiator side");

    let secret = [
        NOISE_PSK_PREFIX.to_vec(),
        remote_pub_key.to_bytes().to_vec(),
    ]
    .concat();
    let secret_hash = Sha256::digest(secret);

    let handshake = Builder::new(pattern.as_str().parse()?)
        .local_private_key(&local_private_key.to_bytes())
        .remote_public_key(&remote_pub_key.to_bytes())
        .psk(pattern.psk_position(), &secret_hash)
        .build_initiator()?;

    let noise_stream = NoiseStream::new(conn, handshake);

    Ok(Connection::Noise(noise_stream.perform_handshake().await?))
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
            NoiseVersion::V1 => {
                upgrade_noise_initiator_v1(
                    conn,
                    config.pattern,
                    config.local_key.private_key(),
                    &key.x25519_pubkey,
                )
                .await
            }
            NoiseVersion::Unknown => {
                error!(
                    "{:?} is announcing an unknown version of Noise",
                    responder_addr
                );
                Err(NoiseError::UnknownVersion)
            }
        },
        None => {
            warn!(
                "{:?} can't speak Noise yet, falling back to TCP",
                responder_addr
            );
            Ok(Connection::Tcp(conn))
        }
    }
}

async fn upgrade_noise_responder_v1(
    conn: TcpStream,
    pattern: NoisePattern,
    local_public_key: &x25519::PublicKey,
    local_private_key: &x25519::PrivateKey,
) -> Result<Connection, NoiseError> {
    trace!("Perform Noise Handshake, responder side");

    let secret = [
        NOISE_PSK_PREFIX.to_vec(),
        local_public_key.to_bytes().to_vec(),
    ]
    .concat();
    let secret_hash = Sha256::digest(secret);

    let handshake = Builder::new(pattern.as_str().parse()?)
        .local_private_key(&local_private_key.to_bytes())
        .psk(pattern.psk_position(), &secret_hash)
        .build_responder()?;

    let noise_stream = NoiseStream::new(conn, handshake);

    Ok(Connection::Noise(noise_stream.perform_handshake().await?))
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
        //responder's info on version is shaky, so initiator has to adapt. This behavior can change in the future
        Some(_) => {
            //Existing node supporting Noise
            upgrade_noise_responder_v1(
                conn,
                config.pattern,
                config.local_key.public_key(),
                config.local_key.private_key(),
            )
            .await
        }
    }
}
