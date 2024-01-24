// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::connection::Connection;
use crate::error::NoiseError;
use crate::stream::{NoisePattern, NoiseStream};
use log::*;
use nym_topology::{NodeVersion, NymTopology};
use sha2::{Digest, Sha256};
use snow::{error::Prerequisite, Builder, Error};
use tokio::net::TcpStream;

pub mod connection;
pub mod error;
pub mod stream;

pub async fn upgrade_noise_initiator(
    conn: TcpStream,
    pattern: NoisePattern,
    local_public_key: Option<&[u8]>,
    local_private_key: &[u8],
    remote_pub_key: &[u8],
    epoch: u32,
) -> Result<Connection, NoiseError> {
    trace!("Perform Noise Handshake, initiator side");

    //In case the local key cannot be known by the remote party, e.g. in a client-gateway connection
    let secret = [
        local_public_key.unwrap_or(&[]),
        remote_pub_key,
        &epoch.to_be_bytes(),
    ]
    .concat();
    let secret_hash = Sha256::digest(secret);

    let handshake = Builder::new(pattern.as_str().parse()?)
        .local_private_key(local_private_key)
        .remote_public_key(remote_pub_key)
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
    local_public_key: &[u8],
    local_private_key: &[u8],
) -> Result<Connection, NoiseError> {
    //Get init material
    let responder_addr = match conn.peer_addr() {
        Ok(addr) => addr,
        Err(err) => {
            error!("Unable to extract peer address from connection - {err}");
            return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
        }
    };
    let (remote_pub_key, version) = match topology.find_node_key_version_by_mix_host(responder_addr)
    {
        Some(res) => (res.0.to_bytes(), res.1),
        None => {
            error!(
                "Cannot find public key for node with address {:?}",
                responder_addr
            );
            return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
        }
    };

    //SW Temporary test
    match version {
        NodeVersion::Explicit(v) if *v < semver::Version::parse("1.2.0").unwrap() => {}
        _ => {
            return Ok(Connection::Tcp(conn));
        }
    }

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
    local_public_key: &[u8],
    local_private_key: &[u8],
    remote_pub_key: Option<&[u8]>,
    epoch: u32,
) -> Result<Connection, NoiseError> {
    trace!("Perform Noise Handshake, responder side");

    //If the remote_key cannot be kwnown, e.g. in a client-gateway connection
    let secret = [
        remote_pub_key.unwrap_or(&[]),
        local_public_key,
        &epoch.to_be_bytes(),
    ]
    .concat();
    let secret_hash = Sha256::digest(secret);

    let handshake = Builder::new(pattern.as_str().parse()?)
        .local_private_key(local_private_key)
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
    local_public_key: &[u8],
    local_private_key: &[u8],
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
    let (remote_pub_key, version) = match topology.find_node_key_version_by_mix_host(initiator_addr)
    {
        Some(res) => (res.0.to_bytes(), res.1),
        None => {
            error!(
                "Cannot find public key for node with address {:?}",
                initiator_addr
            );
            return Err(Error::Prereq(Prerequisite::RemotePublicKey).into());
        }
    };

    //SW Temporary test
    match version {
        NodeVersion::Explicit(v) if *v < semver::Version::parse("1.2.0").unwrap() => {}
        _ => {
            return Ok(Connection::Tcp(conn));
        }
    }

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
