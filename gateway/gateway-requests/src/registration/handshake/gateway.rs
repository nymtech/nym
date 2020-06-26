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

use crate::registration::handshake::shared_key::SharedKey;
use crate::registration::handshake::state::State;
use crate::registration::handshake::{error::HandshakeError, WsItem};
use futures::future::BoxFuture;
use futures::task::{Context, Poll};
use futures::{Future, Sink, Stream};
use rand::{CryptoRng, RngCore};
use std::pin::Pin;
use tokio_tungstenite::tungstenite::Message as WsMessage;

pub(crate) struct GatewayHandshake<'a> {
    handshake_future: BoxFuture<'a, Result<SharedKey, HandshakeError>>,
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
                // init: <- pub_key || g^x
                let (remote_identity, remote_ephemeral_key) =
                    State::<S>::parse_init_message(received_init_payload)?;
                state.update_remote_identity(remote_identity);
                state.derive_shared_key(&remote_ephemeral_key);

                // -> g^y || AES(k, sig(gate_priv, (g^y || g^x))
                let material = state.prepare_key_material_sig(&remote_ephemeral_key);
                state.send_handshake_data(material).await?;

                // <- AES(k, sig(priv, g^x || g^y))
                let remote_key_material = state.receive_handshake_message().await?;
                state.verify_remote_key_material(&remote_key_material, &remote_ephemeral_key)?;
                let finalizer = Self::prepare_finalization_response();

                // -> Ok
                state.send_handshake_data(finalizer).await?;
                Ok(state.finalize_handshake())
            }),
        }
    }

    fn prepare_finalization_response() -> Vec<u8> {
        vec![1]
    }
}

impl<'a> Future for GatewayHandshake<'a> {
    type Output = Result<SharedKey, HandshakeError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.handshake_future).poll(cx)
    }
}
