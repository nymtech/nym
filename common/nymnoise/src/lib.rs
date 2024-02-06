// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::connection::Connection;
use crate::error::NoiseError;
use crate::stream::{NoisePattern, NoiseStream};
use log::*;
use nym_crypto::asymmetric::encryption;
use nym_topology::NymTopology;
use sha2::{Digest, Sha256};
use snow::{error::Prerequisite, Builder, Error};
use tokio::net::TcpStream;

pub mod connection;
pub mod error;
pub mod stream;

pub async fn upgrade_noise_initiator(
    conn: TcpStream,
    pattern: NoisePattern,
    local_public_key: Option<&encryption::PublicKey>,
    local_private_key: &encryption::PrivateKey,
    remote_pub_key: &encryption::PublicKey,
    epoch: u32,
) -> Result<Connection, NoiseError> {
    trace!("Perform Noise Handshake, initiator side");

    //In case the local key cannot be known by the remote party, e.g. in a client-gateway connection
    let secret = [
        local_public_key
            .map(|k| k.to_bytes().to_vec())
            .unwrap_or_default(),
        remote_pub_key.to_bytes().to_vec(),
        epoch.to_be_bytes().to_vec(),
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

pub async fn upgrade_noise_initiator_with_topology(
    conn: TcpStream,
    pattern: NoisePattern,
    topology: &NymTopology,
    epoch: u32,
    local_public_key: &encryption::PublicKey,
    local_private_key: &encryption::PrivateKey,
) -> Result<Connection, NoiseError> {
    //Get init material
    let responder_addr = conn.peer_addr().map_err(|err| {
        error!("Unable to extract peer address from connection - {err}");
        Error::Prereq(Prerequisite::RemotePublicKey)
    })?;

    let remote_pub_key = match topology.find_node_key_by_mix_host(responder_addr) {
        Ok(Some(key)) => encryption::PublicKey::from_base58_string(key)?,
        Ok(None) => {
            warn!(
                "{:?} can't speak Noise yet, falling back to TCP",
                responder_addr
            );
            return Ok(Connection::Tcp(conn));
        }
        Err(_) => {
            error!(
                "Cannot find public key for node with address {:?}",
                responder_addr
            ); //Do we still pursue a TCP connection or not?
            return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
        }
    };

    upgrade_noise_initiator(
        conn,
        pattern,
        Some(local_public_key),
        local_private_key,
        &remote_pub_key,
        epoch,
    )
    .await
}

pub async fn upgrade_noise_responder(
    conn: TcpStream,
    pattern: NoisePattern,
    local_public_key: &encryption::PublicKey,
    local_private_key: &encryption::PrivateKey,
    remote_pub_key: Option<&encryption::PublicKey>,
    epoch: u32,
) -> Result<Connection, NoiseError> {
    trace!("Perform Noise Handshake, responder side");

    //If the remote_key cannot be kwnown, e.g. in a client-gateway connection
    let secret = [
        remote_pub_key
            .map(|k| k.to_bytes().to_vec())
            .unwrap_or_default(),
        local_public_key.to_bytes().to_vec(),
        epoch.to_be_bytes().to_vec(),
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

pub async fn upgrade_noise_responder_with_topology(
    conn: TcpStream,
    pattern: NoisePattern,
    topology: &NymTopology,
    epoch: u32,
    local_public_key: &encryption::PublicKey,
    local_private_key: &encryption::PrivateKey,
) -> Result<Connection, NoiseError> {
    //Get init material
    let initiator_addr = match conn.peer_addr() {
        Ok(addr) => addr,
        Err(err) => {
            error!("Unable to extract peer address from connection - {err}");
            return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
        }
    };

    //SW : for private gateway, we could try to perform the handshake without that key?
    let remote_pub_key = match topology.find_node_key_by_mix_host(initiator_addr) {
        Ok(Some(key)) => encryption::PublicKey::from_base58_string(key)?,
        Ok(None) => {
            warn!(
                "{:?} can't speak Noise yet, falling back to TCP",
                initiator_addr
            );
            return Ok(Connection::Tcp(conn));
        }
        Err(_) => {
            error!(
                "Cannot find public key for node with address {:?}",
                initiator_addr
            ); //Do we still pursue a TCP connection with that node or not?
            return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
        }
    };

    upgrade_noise_responder(
        conn,
        pattern,
        local_public_key,
        local_private_key,
        Some(&remote_pub_key),
        epoch,
    )
    .await
}
