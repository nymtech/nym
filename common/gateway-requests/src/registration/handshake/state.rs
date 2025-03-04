// Copyright 2020-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::registration::handshake::error::HandshakeError;
use crate::registration::handshake::messages::{
    HandshakeMessage, Initialisation, MaterialExchange,
};
use crate::registration::handshake::{WsItem, KDF_SALT_LENGTH};
use crate::shared_key::SharedKeySize;
use crate::{types, SharedSymmetricKey, AES_GCM_SIV_PROTOCOL_VERSION};
use futures::{Sink, SinkExt, Stream, StreamExt};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_crypto::symmetric::aead::random_nonce;
use nym_crypto::{
    asymmetric::{encryption, identity},
    generic_array::typenum::Unsigned,
    hkdf,
};
use nym_sphinx::params::{GatewayEncryptionAlgorithm, GatewaySharedKeyHkdfAlgorithm};
use rand::{thread_rng, CryptoRng, RngCore};
use std::any::{type_name, Any};
use std::str::FromStr;
use std::time::Duration;
use tracing::log::*;
use tungstenite::Message as WsMessage;

#[cfg(not(target_arch = "wasm32"))]
use nym_task::TaskClient;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::timeout;

#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::timeout;

/// Handshake state.
pub(crate) struct State<'a, S, R> {
    /// The underlying WebSocket stream.
    ws_stream: &'a mut S,

    /// Pseudorandom number generator used during the exchange
    rng: &'a mut R,

    /// Identity of the local "node" (client or gateway) which is used
    /// during the handshake.
    identity: &'a ed25519::KeyPair,

    /// Local ephemeral Diffie-Hellman keypair generated as a part of the handshake.
    ephemeral_keypair: x25519::KeyPair,

    /// The derived shared key using the ephemeral keys of both parties.
    derived_shared_keys: Option<SharedSymmetricKey>,

    /// The known or received public identity key of the remote.
    /// Ideally it would always be known before the handshake was initiated.
    remote_pubkey: Option<ed25519::PublicKey>,

    // this field is really out of place here, however, we need to propagate this information somehow
    // in order to establish correct protocol for backwards compatibility reasons
    expects_credential_usage: bool,

    // channel to receive shutdown signal
    #[cfg(not(target_arch = "wasm32"))]
    shutdown: TaskClient,
}

impl<'a, S, R> State<'a, S, R> {
    pub(crate) fn new(
        rng: &'a mut R,
        ws_stream: &'a mut S,
        identity: &'a identity::KeyPair,
        remote_pubkey: Option<identity::PublicKey>,
        #[cfg(not(target_arch = "wasm32"))] shutdown: TaskClient,
    ) -> Self
    where
        R: CryptoRng + RngCore,
    {
        let ephemeral_keypair = encryption::KeyPair::new(rng);
        State {
            ws_stream,
            rng,
            ephemeral_keypair,
            identity,
            remote_pubkey,
            derived_shared_keys: None,
            // later on this should become the default
            expects_credential_usage: false,
            #[cfg(not(target_arch = "wasm32"))]
            shutdown,
        }
    }

    pub(crate) fn with_credential_usage(mut self, expects_credential_usage: bool) -> Self {
        self.expects_credential_usage = expects_credential_usage;
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn local_ephemeral_key(&self) -> &encryption::PublicKey {
        self.ephemeral_keypair.public_key()
    }

    pub(crate) fn generate_initiator_salt(&mut self) -> Vec<u8>
    where
        R: CryptoRng + RngCore,
    {
        let mut salt = vec![0u8; KDF_SALT_LENGTH];
        self.rng.fill_bytes(&mut salt);
        salt
    }

    // LOCAL_ID_PUBKEY || EPHEMERAL_KEY || MAYBE_SALT
    // Eventually the ID_PUBKEY prefix will get removed and recipient will know
    // initializer's identity from another source.
    pub(crate) fn init_message(&self, initiator_salt: Vec<u8>) -> Initialisation {
        Initialisation {
            identity: *self.identity.public_key(),
            ephemeral_dh: *self.ephemeral_keypair.public_key(),
            initiator_salt,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn finalization_message(
        &self,
    ) -> crate::registration::handshake::messages::Finalization {
        crate::registration::handshake::messages::Finalization { success: true }
    }

    pub(crate) fn derive_shared_key(
        &mut self,
        remote_ephemeral_key: &encryption::PublicKey,
        initiator_salt: &[u8],
    ) {
        let dh_result = self
            .ephemeral_keypair
            .private_key()
            .diffie_hellman(remote_ephemeral_key);

        let key_size = SharedKeySize::to_usize();

        // there is no reason for this to fail as our okm is expected to be only 16 bytes
        let okm = hkdf::extract_then_expand::<GatewaySharedKeyHkdfAlgorithm>(
            Some(initiator_salt),
            &dh_result,
            None,
            key_size,
        )
        .expect("somehow too long okm was provided");

        let shared_key = SharedSymmetricKey::try_from_bytes(&okm)
            .expect("okm was expanded to incorrect length!");

        self.derived_shared_keys = Some(shared_key)
    }

    // produces AES(k, SIG(ID_PRIV, G^x || G^y),
    // assuming x is local and y is remote
    pub(crate) fn prepare_key_material_sig(
        &self,
        remote_ephemeral_key: &encryption::PublicKey,
    ) -> Result<MaterialExchange, HandshakeError> {
        let plaintext: Vec<_> = self
            .ephemeral_keypair
            .public_key()
            .to_bytes()
            .into_iter()
            .chain(remote_ephemeral_key.to_bytes())
            .collect();
        let signature = self.identity.private_key().sign(plaintext);

        let mut rng = thread_rng();
        let nonce = random_nonce::<GatewayEncryptionAlgorithm, _>(&mut rng);

        // SAFETY: this function is only called after the local key has already been derived
        let signature_ciphertext = self
            .derived_shared_keys
            .as_ref()
            .expect("shared key was not derived!")
            .encrypt(&signature.to_bytes(), &nonce)?;

        Ok(MaterialExchange {
            signature_ciphertext,
            nonce,
        })
    }

    pub(crate) fn verify_remote_key_material(
        &self,
        remote_response: &MaterialExchange,
        remote_ephemeral_key: &x25519::PublicKey,
    ) -> Result<(), HandshakeError> {
        // SAFETY: this function is only called after the local key has already been derived
        let derived_shared_key = self
            .derived_shared_keys
            .as_ref()
            .expect("shared key was not derived!");

        // first decrypt received data
        let decrypted_signature = derived_shared_key.decrypt(
            &remote_response.signature_ciphertext,
            &remote_response.nonce,
        )?;

        // now verify signature itself
        let signature = identity::Signature::from_bytes(&decrypted_signature)
            .map_err(|_| HandshakeError::InvalidSignature)?;

        // g^y || g^x, if y is remote and x is local
        let signed_payload: Vec<_> = remote_ephemeral_key
            .to_bytes()
            .into_iter()
            .chain(self.ephemeral_keypair.public_key().to_bytes())
            .collect();

        self.remote_pubkey
            .as_ref()
            .unwrap()
            .verify(signed_payload, &signature)
            .map_err(|_| HandshakeError::InvalidSignature)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn update_remote_identity(&mut self, remote_pubkey: identity::PublicKey) {
        self.remote_pubkey = Some(remote_pubkey)
    }

    fn on_wg_msg(msg: Option<WsItem>) -> Result<Option<Vec<u8>>, HandshakeError> {
        let Some(msg) = msg else {
            return Err(HandshakeError::ClosedStream);
        };

        let Ok(msg) = msg else {
            return Err(HandshakeError::NetworkError);
        };
        match msg {
            WsMessage::Text(ref ws_msg) => {
                match types::RegistrationHandshake::from_str(ws_msg) {
                    Ok(reg_handshake_msg) => {
                        match reg_handshake_msg {
                            // hehe, that's a bit disgusting that the type system requires we explicitly ignore the
                            // protocol_version field that we actually never attach at this point
                            // yet another reason for the overdue refactor
                            types::RegistrationHandshake::HandshakePayload { data, .. } => {
                                Ok(Some(data))
                            }
                            types::RegistrationHandshake::HandshakeError { message } => {
                                Err(HandshakeError::RemoteError(message))
                            }
                        }
                    }
                    Err(_) => {
                        error!("Received a non-handshake message during the registration handshake! It's getting dropped. The received content was: '{msg}'");
                        Ok(None)
                    }
                }
            }
            _ => {
                error!("Received non-text message during registration handshake");
                Ok(None)
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn _receive_handshake_message_bytes(&mut self) -> Result<Vec<u8>, HandshakeError>
    where
        S: Stream<Item = WsItem> + Unpin,
    {
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.recv() => return Err(HandshakeError::ReceivedShutdown),
                msg = self.ws_stream.next() => {
                    let Some(ret) = Self::on_wg_msg(msg)? else {
                        continue;
                    };
                    return Ok(ret);
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    async fn _receive_handshake_message_bytes(&mut self) -> Result<Vec<u8>, HandshakeError>
    where
        S: Stream<Item = WsItem> + Unpin,
    {
        loop {
            let msg = self.ws_stream.next().await;
            let Some(ret) = Self::on_wg_msg(msg)? else {
                continue;
            };
            return Ok(ret);
        }
    }

    pub(crate) async fn receive_handshake_message<M>(&mut self) -> Result<M, HandshakeError>
    where
        S: Stream<Item = WsItem> + Unpin,
        M: HandshakeMessage,
    {
        // TODO: make timeout duration configurable
        let bytes = timeout(
            Duration::from_secs(5),
            self._receive_handshake_message_bytes(),
        )
        .await
        .map_err(|_| HandshakeError::Timeout)??;

        M::try_from_bytes(&bytes)
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

    fn request_protocol_version(&self) -> u8 {
        AES_GCM_SIV_PROTOCOL_VERSION
    }

    pub(crate) async fn send_handshake_data<M>(
        &mut self,
        inner_message: M,
    ) -> Result<(), HandshakeError>
    where
        S: Sink<WsMessage> + Unpin,
        M: HandshakeMessage + Any,
    {
        trace!("sending handshake message: {}", type_name::<M>());

        let handshake_message = types::RegistrationHandshake::new_payload(
            inner_message.into_bytes(),
            self.request_protocol_version(),
        );
        self.ws_stream
            .send(WsMessage::Text(handshake_message.try_into().unwrap()))
            .await
            .map_err(|_| HandshakeError::ClosedStream)
    }

    /// Finish the handshake, yielding the derived shared key and implicitly dropping all borrowed
    /// values.
    pub(crate) fn finalize_handshake(self) -> SharedSymmetricKey {
        self.derived_shared_keys.unwrap()
    }

    // If any step along the way failed (that are non-network related),
    // try to send 'error' message to the remote
    // party to indicate handshake should be terminated
    pub(crate) async fn check_for_handshake_processing_error<T>(
        &mut self,
        result: Result<T, HandshakeError>,
    ) -> Result<T, HandshakeError>
    where
        S: Sink<WsMessage> + Unpin,
    {
        match result {
            Ok(ok) => Ok(ok),
            Err(err) => {
                self.send_handshake_error(err.to_string()).await?;
                Err(err)
            }
        }
    }
}
