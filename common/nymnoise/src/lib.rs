// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::ed25519;
use nym_noise_keys::NoiseVersion;
use nympsq::psq::CONTEXT_LEN;

use snow::{error::Prerequisite, Error};
use tokio::net::TcpStream;
use tracing::{error, warn};

pub mod config;
pub mod connection;
pub mod error;
pub mod stream;

use crate::{
    config::NoiseConfig, connection::Connection, error::NoiseError, stream::NoiseStreamBuilder,
};

const NOISE_PSK_PREFIX: &[u8] = b"NYMTECH_NOISE_dQw4w9WgXcQ";

pub(crate) const NOISE_PSQ_DEFAULT_CONTEXT: &'static [u8; CONTEXT_LEN] = b"Exsl88AD2ccS99kk";
pub(crate) const NOISE_PSQ_DEFAULT_DURATION_SECS: u64 = 1000;

pub const LATEST_NOISE_VERSION: NoiseVersion = NoiseVersion::V2;

// TODO: this should be behind some trait because presumably, depending on the version,
// other arguments would be needed
mod psk_gen {
    use std::time::Duration;

    use crate::{error::NoiseError, stream::Psk, NOISE_PSK_PREFIX};
    use libcrux_ed25519::VerificationKey;
    use libcrux_kem::{PrivateKey, PublicKey};
    use libcrux_psq::impls::X25519;
    use nym_crypto::asymmetric::ed25519;

    use nympsq::{
        error::PSQError,
        psq::{PSQInitiator, PSQResponder, CONTEXT_LEN, PSK_HANDLE_LEN},
    };

    use sha2::{Digest, Sha256};

    pub fn generate_psk_v1(responder_pub_key: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(NOISE_PSK_PREFIX);
        hasher.update(responder_pub_key);
        hasher.finalize().into()
    }

    pub(crate) fn psq_respond_x25519(
        responder_private_key: impl AsRef<[u8]>,
        responder_public_key: impl AsRef<[u8]>,
        initiator_verification_key: &ed25519::PublicKey,
        initiator_message: &mut [u8],
        context: &[u8; CONTEXT_LEN],
        psq_ttl: Duration,
        psk_handle: &[u8; PSK_HANDLE_LEN],
    ) -> Result<(Psk, Vec<u8>), PSQError> {
        let kem_private_key = PrivateKey::decode(
            libcrux_kem::Algorithm::X25519,
            responder_private_key.as_ref(),
        )?;
        let kem_public_key = PublicKey::decode(
            libcrux_kem::Algorithm::X25519,
            responder_public_key.as_ref(),
        )?;

        let responder: PSQResponder<X25519> = PSQResponder::init(&kem_private_key, &kem_public_key);

        responder.compute_responder_message(
            &VerificationKey::from_bytes(initiator_verification_key.to_bytes()),
            initiator_message,
            context,
            psq_ttl,
            psk_handle,
        )
    }

    pub(crate) fn psq_initiate_x25519(
        initiator: &mut PSQInitiator<X25519>,
        responder_pub_key: impl AsRef<[u8]>,
        context: &[u8; CONTEXT_LEN],
        psq_ttl: Duration,
    ) -> Result<Vec<u8>, NoiseError> {
        let pub_key =
            PublicKey::decode(libcrux_kem::Algorithm::X25519, responder_pub_key.as_ref())?;
        Ok(initiator.compute_initiator_message(&mut rand::rng(), &pub_key, context, psq_ttl)?)
    }
}

pub async fn upgrade_noise_initiator(
    conn: TcpStream,
    config: &NoiseConfig,
    initiator_identity_keypair: Option<ed25519::KeyPair>,
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
        NoiseVersion::V2 => NoiseVersion::V2,

        // We're talking to a more recent node, but we can't adapt. Let's try to do our best and if it fails, it fails.
        // If that node sees we're older, it will try to adapt too.
        NoiseVersion::Unknown(version) => {
            warn!("{responder_addr} is announcing an v{version} version of Noise that we don't know how to parse, we will attempt to downgrade to our current highest supported version");
            LATEST_NOISE_VERSION
        }
    };

    let (signing_key, verification_key): (Option<[u8; 32]>, Option<[u8; 32]>) =
        match initiator_identity_keypair {
            Some(keypair) => (
                Some(keypair.private_key().to_bytes()),
                Some(keypair.public_key().to_bytes()),
            ),
            None => (None, None),
        };
    NoiseStreamBuilder::new(conn)
        .perform_initiator_handshake(
            config,
            handshake_version,
            key.x25519_pubkey,
            signing_key,
            verification_key,
        )
        .await
        .map(|stream| Connection::Noise(Box::new(stream)))
}
pub async fn upgrade_noise_responder(
    conn: TcpStream,
    config: &NoiseConfig,
    initiator_verification_key: Option<ed25519::PublicKey>,
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
        .perform_responder_handshake(config, initiator_verification_key)
        .await
        .map(|stream| Connection::Noise(Box::new(stream)))
}
