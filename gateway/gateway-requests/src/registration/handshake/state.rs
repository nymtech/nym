// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::registration::handshake::error::HandshakeError;
use crate::registration::handshake::shared_key::{SharedKeySize, SharedKeys};
use crate::registration::handshake::WsItem;
use crate::types;
use crypto::{
    asymmetric::{encryption, identity},
    generic_array::typenum::Unsigned,
    hkdf,
    symmetric::stream_cipher,
};
use futures::{Sink, SinkExt, Stream, StreamExt};
use log::*;
use nymsphinx::params::{GatewayEncryptionAlgorithm, GatewaySharedKeyHkdfAlgorithm};
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use tungstenite::Message as WsMessage;

#[derive(Serialize, Deserialize)]
pub struct InitMessage {
    local_id_pubkey: [u8; identity::PUBLIC_KEY_LENGTH],
    ephemeral_key: [u8; identity::PUBLIC_KEY_LENGTH],
}

impl InitMessage {
    fn new(local_id_pubkey: &identity::PublicKey, ephemeral_key: &encryption::PublicKey) -> Self {
        InitMessage {
            local_id_pubkey: local_id_pubkey.to_bytes(),
            ephemeral_key: ephemeral_key.to_bytes(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn local_id_pubkey(&self) -> identity::PublicKey {
        identity::PublicKey::from_bytes(&self.local_id_pubkey).unwrap()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn ephemeral_key(&self) -> encryption::PublicKey {
        encryption::PublicKey::from_bytes(&self.ephemeral_key).unwrap()
    }

    fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}

impl TryFrom<&[u8]> for InitMessage {
    type Error = HandshakeError;

    fn try_from(value: &[u8]) -> Result<InitMessage, Self::Error> {
        bincode::deserialize(value).map_err(|e| e.into())
    }
}
/// Handshake state.
pub(crate) struct State<'a, S> {
    /// The underlying WebSocket stream.
    ws_stream: &'a mut S,

    /// Identity of the local "node" (client or gateway) which is used
    /// during the handshake.
    identity: &'a identity::KeyPair,

    /// Local ephemeral Diffie-Hellman keypair generated as a part of the handshake.
    ephemeral_keypair: encryption::KeyPair,

    /// The derived shared key using the ephemeral keys of both parties.
    derived_shared_keys: Option<SharedKeys>,

    /// The known or received public identity key of the remote.
    /// Ideally it would always be known before the handshake was initiated.
    remote_pubkey: Option<identity::PublicKey>,
}

impl<'a, S> State<'a, S> {
    pub(crate) fn new(
        rng: &mut (impl RngCore + CryptoRng),
        ws_stream: &'a mut S,
        identity: &'a identity::KeyPair,
        remote_pubkey: Option<identity::PublicKey>,
    ) -> Self {
        let ephemeral_keypair = encryption::KeyPair::new(rng);
        State {
            ws_stream,
            ephemeral_keypair,
            identity,
            remote_pubkey,
            derived_shared_keys: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn local_ephemeral_key(&self) -> &encryption::PublicKey {
        self.ephemeral_keypair.public_key()
    }

    // LOCAL_ID_PUBKEY || EPHEMERAL_KEY
    // Eventually the ID_PUBKEY prefix will get removed and recipient will know
    // initializer's identity from another source.
    pub(crate) fn init_message(&self) -> Vec<u8> {
        InitMessage::new(
            self.identity.public_key(),
            self.ephemeral_keypair.public_key(),
        )
        .to_bytes()
    }

    // this will need to be adjusted when REMOTE_ID_PUBKEY is removed
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn parse_init_message(init_message: Vec<u8>) -> Result<InitMessage, HandshakeError> {
        InitMessage::try_from(init_message.as_slice()).map_err(|_| HandshakeError::MalformedRequest)
    }

    pub(crate) fn derive_shared_key(&mut self, remote_ephemeral_key: &encryption::PublicKey) {
        let dh_result = self
            .ephemeral_keypair
            .private_key()
            .diffie_hellman(remote_ephemeral_key);

        // there is no reason for this to fail as our okm is expected to be only 16 bytes
        let okm = hkdf::extract_then_expand::<GatewaySharedKeyHkdfAlgorithm>(
            None,
            &dh_result,
            None,
            SharedKeySize::to_usize(),
        )
        .expect("somehow too long okm was provided");

        let derived_shared_key =
            SharedKeys::try_from_bytes(&okm).expect("okm was expanded to incorrect length!");

        self.derived_shared_keys = Some(derived_shared_key)
    }

    // produces AES(k, SIG(ID_PRIV, G^x || G^y),
    // assuming x is local and y is remote
    pub(crate) fn prepare_key_material_sig(
        &self,
        remote_ephemeral_key: &encryption::PublicKey,
    ) -> Vec<u8> {
        let message: Vec<_> = self
            .ephemeral_keypair
            .public_key()
            .to_bytes()
            .iter()
            .cloned()
            .chain(remote_ephemeral_key.to_bytes().iter().cloned())
            .collect();

        let signature = self.identity.private_key().sign(&message);
        let zero_iv = stream_cipher::zero_iv::<GatewayEncryptionAlgorithm>();
        stream_cipher::encrypt::<GatewayEncryptionAlgorithm>(
            self.derived_shared_keys.as_ref().unwrap().encryption_key(),
            &zero_iv,
            &signature.to_bytes(),
        )
    }

    // must be called after shared key was derived locally and remote's identity is known
    pub(crate) fn verify_remote_key_material(
        &self,
        remote_material: &[u8],
        remote_ephemeral_key: &encryption::PublicKey,
    ) -> Result<(), HandshakeError> {
        if remote_material.len() != identity::SIGNATURE_LENGTH {
            return Err(HandshakeError::KeyMaterialOfInvalidSize(
                remote_material.len(),
            ));
        }
        let derived_shared_key = self
            .derived_shared_keys
            .as_ref()
            .expect("shared key was not derived!");

        // first decrypt received data
        let zero_iv = stream_cipher::zero_iv::<GatewayEncryptionAlgorithm>();
        let decrypted_signature = stream_cipher::decrypt::<GatewayEncryptionAlgorithm>(
            derived_shared_key.encryption_key(),
            &zero_iv,
            remote_material,
        );

        // now verify signature itself
        let signature = identity::Signature::from_bytes(&decrypted_signature)
            .map_err(|_| HandshakeError::InvalidSignature)?;

        // g^y || g^x, if y is remote and x is local
        let signed_payload: Vec<_> = remote_ephemeral_key
            .to_bytes()
            .iter()
            .cloned()
            .chain(
                self.ephemeral_keypair
                    .public_key()
                    .to_bytes()
                    .iter()
                    .cloned(),
            )
            .collect();

        self.remote_pubkey
            .as_ref()
            .unwrap()
            .verify(&signed_payload, &signature)
            .map_err(|_| HandshakeError::InvalidSignature)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn update_remote_identity(&mut self, remote_pubkey: identity::PublicKey) {
        self.remote_pubkey = Some(remote_pubkey)
    }

    pub(crate) async fn receive_handshake_message(&mut self) -> Result<Vec<u8>, HandshakeError>
    where
        S: Stream<Item = WsItem> + Unpin,
    {
        loop {
            if let Some(msg) = self.ws_stream.next().await {
                if let Ok(msg) = msg {
                    match msg {
                        WsMessage::Text(ws_msg) => match types::RegistrationHandshake::try_from(ws_msg) {
                            Ok(reg_handshake_msg) => return match reg_handshake_msg {
                                types::RegistrationHandshake::HandshakePayload { data } => Ok(data),
                                types::RegistrationHandshake::HandshakeError { message } => Err(HandshakeError::RemoteError(message)),
                            },
                            Err(_) => error!("Received a non-handshake message during the registration handshake! It's getting dropped."),
                        },
                        _ => error!("Received non-text message during registration handshake"),
                    }
                } else {
                    return Err(HandshakeError::NetworkError);
                }
            } else {
                return Err(HandshakeError::ClosedStream);
            }
        }
    }

    // upon receiving this, the receiver should terminate the handshake
    pub(crate) async fn send_handshake_error<M: Into<String>>(
        &mut self,
        message: M,
    ) -> Result<(), HandshakeError>
    where
        S: Sink<WsMessage> + Unpin,
    {
        let handshake_message = types::RegistrationHandshake::new_error(message);
        self.ws_stream
            .send(WsMessage::Text(handshake_message.try_into().unwrap()))
            .await
            .map_err(|_| HandshakeError::ClosedStream)
    }

    pub(crate) async fn send_handshake_data(
        &mut self,
        payload: Vec<u8>,
    ) -> Result<(), HandshakeError>
    where
        S: Sink<WsMessage> + Unpin,
    {
        let handshake_message = types::RegistrationHandshake::new_payload(payload);
        self.ws_stream
            .send(WsMessage::Text(handshake_message.try_into().unwrap()))
            .await
            .map_err(|_| HandshakeError::ClosedStream)
    }

    /// Finish the handshake, yielding the derived shared key and implicitly dropping all borrowed
    /// values.
    pub(crate) fn finalize_handshake(self) -> SharedKeys {
        self.derived_shared_keys.unwrap()
    }
}
