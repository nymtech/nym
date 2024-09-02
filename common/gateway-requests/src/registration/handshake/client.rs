// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::registration::handshake::shared_key::SharedKeys;
use crate::registration::handshake::state::State;
use crate::registration::handshake::{error::HandshakeError, WsItem};
use futures::future::BoxFuture;
use futures::task::{Context, Poll};
use futures::{Future, Sink, Stream};
use nym_crypto::asymmetric::encryption::PUBLIC_KEY_SIZE;
use nym_crypto::asymmetric::identity::SIGNATURE_LENGTH;
use nym_crypto::asymmetric::{encryption, identity};
use nym_task::TaskClient;
use rand::{CryptoRng, RngCore};
use std::pin::Pin;
use tungstenite::Message as WsMessage;

pub(crate) struct ClientHandshake<'a> {
    handshake_future: BoxFuture<'a, Result<SharedKeys, HandshakeError>>,
}

impl<'a> ClientHandshake<'a> {
    pub(crate) fn new<S>(
        rng: &mut (impl RngCore + CryptoRng),
        ws_stream: &'a mut S,
        identity: &'a nym_crypto::asymmetric::identity::KeyPair,
        gateway_pubkey: identity::PublicKey,
        expects_credential_usage: bool,
        shutdown: TaskClient,
    ) -> Self
    where
        S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin + Send + 'a,
    {
        let mut state = State::new(
            rng,
            ws_stream,
            identity,
            Some(gateway_pubkey),
            expects_credential_usage,
            shutdown,
        );

        ClientHandshake {
            handshake_future: Box::pin(async move {
                // If any step along the way failed (that are non-network related),
                // try to send 'error' message to the remote
                // party to indicate handshake should be terminated
                pub(crate) async fn check_processing_error<T, S>(
                    result: Result<T, HandshakeError>,
                    state: &mut State<'_, S>,
                ) -> Result<T, HandshakeError>
                where
                    S: Sink<WsMessage> + Unpin,
                {
                    match result {
                        Ok(ok) => Ok(ok),
                        Err(err) => {
                            state.send_handshake_error(err.to_string()).await?;
                            Err(err)
                        }
                    }
                }

                let init_message = state.init_message();
                state.send_handshake_data(init_message).await?;

                // <- g^y || AES(k, sig(gate_priv, (g^y || g^x))
                let mid_res = state.receive_handshake_message().await?;
                let (remote_ephemeral_key, remote_key_material) =
                    check_processing_error(Self::parse_mid_response(mid_res), &mut state).await?;

                // hkdf::<blake3>::(g^xy)
                state.derive_shared_key(&remote_ephemeral_key);
                let verification_res =
                    state.verify_remote_key_material(&remote_key_material, &remote_ephemeral_key);
                check_processing_error(verification_res, &mut state).await?;

                // AES(k, sig(client_priv, (g^y || g^x))
                let material = state.prepare_key_material_sig(&remote_ephemeral_key);

                // -> AES(k, sig(client_priv, g^x || g^y))
                state.send_handshake_data(material).await?;

                // <- Ok
                let finalization = state.receive_handshake_message().await?;
                check_processing_error(Self::parse_finalization_response(finalization), &mut state)
                    .await?;
                Ok(state.finalize_handshake())
            }),
        }
    }

    // client should have received
    // G^y || AES(k, SIG(PRIV_GATE, G^y || G^x))
    fn parse_mid_response(
        mut resp: Vec<u8>,
    ) -> Result<(encryption::PublicKey, Vec<u8>), HandshakeError> {
        if resp.len() != PUBLIC_KEY_SIZE + SIGNATURE_LENGTH {
            return Err(HandshakeError::MalformedResponse);
        }

        let remote_key_material = resp.split_off(PUBLIC_KEY_SIZE);
        // this can only fail if the provided bytes have len different from PUBLIC_KEY_SIZE
        // which is impossible
        let remote_ephemeral_key = encryption::PublicKey::from_bytes(&resp).unwrap();
        Ok((remote_ephemeral_key, remote_key_material))
    }

    fn parse_finalization_response(resp: Vec<u8>) -> Result<(), HandshakeError> {
        if resp.len() != 1 {
            return Err(HandshakeError::MalformedResponse);
        }
        if resp[0] == 1 {
            Ok(())
        } else if resp[0] == 0 {
            Err(HandshakeError::HandshakeFailure)
        } else {
            Err(HandshakeError::MalformedResponse)
        }
    }
}

impl<'a> Future for ClientHandshake<'a> {
    type Output = Result<SharedKeys, HandshakeError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.handshake_future).poll(cx)
    }
}
