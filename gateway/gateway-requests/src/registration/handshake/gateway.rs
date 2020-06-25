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

use crate::registration::handshake::state::State;
use crate::registration::handshake::{
    error::HandshakeError, DerivedSharedKey, RegistrationHandshake, WsItem,
};
use futures::future::BoxFuture;
use futures::task::{Context, Poll};
use futures::{Future, Sink, Stream};
use rand::{CryptoRng, RngCore};
use std::marker::PhantomData;
use std::pin::Pin;
use tokio_tungstenite::tungstenite::{Error as WsError, Message as WsMessage};

pub(crate) struct GatewayHandshake<'a, S> {
    // same could have been achieved via futures::future::BoxFuture, but this way we don't
    // need to specify redundant lifetimes
    handshake_future: BoxFuture<'a, Result<DerivedSharedKey, HandshakeError>>,
    _phantom: PhantomData<&'a S>,
}

// impl<'a, S> RegistrationHandshake<S> for GatewayHandshake<'a, S> {}

impl<'a, S> GatewayHandshake<'a, S> {
    pub(crate) fn new(
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
            _phantom: PhantomData,
        }
    }

    fn prepare_finalization_response() -> Vec<u8> {
        vec![1]
    }

    // // client should have received
    // // G^y || AES(k, SIG(PRIV_S, G^y || G^x))
    // fn parse_mid_response(
    //     mut resp: Vec<u8>,
    // ) -> Result<(encryption::PublicKey, Vec<u8>), HandshakeError> {
    //     if resp.len() != PUBLIC_KEY_SIZE + SIGNATURE_LENGTH {
    //         return Err(HandshakeError::MalformedResponse);
    //     }
    //
    //     let remote_key_material = resp.split_off(PUBLIC_KEY_SIZE);
    //     // this can only fail if the provided bytes have len different from PUBLIC_KEY_SIZE
    //     // which is impossible
    //     let remote_ephemeral_key = encryption::PublicKey::from_bytes(&resp).unwrap();
    //     Ok((remote_ephemeral_key, remote_key_material))
    // }
    //
    // fn parse_finalization_response(resp: Vec<u8>) -> Result<(), HandshakeError> {
    //     if resp.len() != 1 {
    //         return Err(HandshakeError::MalformedResponse);
    //     }
    //     if resp[0] == 1 {
    //         Ok(())
    //     } else if resp[0] == 0 {
    //         Err(HandshakeError::HandshakeFailure)
    //     } else {
    //         Err(HandshakeError::MalformedResponse)
    //     }
    // }
}

impl<'a, S> Future for GatewayHandshake<'a, S> {
    type Output = Result<DerivedSharedKey, HandshakeError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.handshake_future).poll(cx)
    }
}
