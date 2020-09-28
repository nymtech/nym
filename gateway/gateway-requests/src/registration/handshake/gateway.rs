// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::registration::handshake::shared_key::SharedKeys;
use crate::registration::handshake::state::State;
use crate::registration::handshake::{error::HandshakeError, WsItem};
use crypto::asymmetric::encryption;
use futures::future::BoxFuture;
use futures::task::{Context, Poll};
use futures::{Future, Sink, Stream};
use rand::{CryptoRng, RngCore};
use std::pin::Pin;
use tungstenite::Message as WsMessage;

pub(crate) struct GatewayHandshake<'a> {
    handshake_future: BoxFuture<'a, Result<SharedKeys, HandshakeError>>,
}

impl<'a> GatewayHandshake<'a> {
    pub(crate) fn new<S>(
        rng: &mut (impl RngCore + CryptoRng),
        ws_stream: &'a mut S,
        identity: &'a crypto::asymmetric::identity::KeyPair,
        received_init_payload: Vec<u8>,
    ) -> Self
    where
        S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin + Send + 'a,
    {
        let mut state = State::new(rng, ws_stream, identity, None);
        GatewayHandshake {
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

                // init: <- pub_key || g^x
                let (remote_identity, remote_ephemeral_key) = check_processing_error(
                    State::<S>::parse_init_message(received_init_payload),
                    &mut state,
                )
                .await?;
                state.update_remote_identity(remote_identity);

                // hkdf::<blake3>::(g^xy)
                state.derive_shared_key(&remote_ephemeral_key);

                // AES(k, sig(gate_priv, (g^y || g^x))
                let material = state.prepare_key_material_sig(&remote_ephemeral_key);

                // g^y || AES(k, sig(gate_priv, (g^y || g^x))
                let handshake_payload = Self::combine_material_with_ephemeral_key(
                    state.local_ephemeral_key(),
                    material,
                );

                // -> g^y || AES(k, sig(gate_priv, (g^y || g^x))
                state.send_handshake_data(handshake_payload).await?;

                // <- AES(k, sig(client_priv, g^x || g^y))
                let remote_key_material = state.receive_handshake_message().await?;
                let verification_res =
                    state.verify_remote_key_material(&remote_key_material, &remote_ephemeral_key);
                check_processing_error(verification_res, &mut state).await?;
                let finalizer = Self::prepare_finalization_response();

                // -> Ok
                state.send_handshake_data(finalizer).await?;
                Ok(state.finalize_handshake())
            }),
        }
    }

    // create g^y || AES(k, sig(gate_priv, (g^y || g^x))
    fn combine_material_with_ephemeral_key(
        ephemeral_key: &encryption::PublicKey,
        material: Vec<u8>,
    ) -> Vec<u8> {
        ephemeral_key
            .to_bytes()
            .iter()
            .cloned()
            .chain(material.into_iter())
            .collect()
    }

    fn prepare_finalization_response() -> Vec<u8> {
        vec![1]
    }
}

impl<'a> Future for GatewayHandshake<'a> {
    type Output = Result<SharedKeys, HandshakeError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.handshake_future).poll(cx)
    }
}
