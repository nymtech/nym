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

use self::client::ClientHandshake;
use self::error::HandshakeError;
use self::gateway::GatewayHandshake;
use crate::types;
use crypto::asymmetric::{encryption, identity};
use crypto::symmetric::aes_ctr;
use crypto::symmetric::aes_ctr::generic_array::GenericArray;
use crypto::symmetric::aes_ctr::Aes128Key;
use futures::task::{Context, Poll};
use futures::{Future, Sink, SinkExt, Stream, StreamExt};
use log::*;
use rand::rngs::OsRng;
use rand::{CryptoRng, RngCore};
use serde::export::Formatter;
use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Display};
use tokio_tungstenite::tungstenite::{Error as WsError, Message as WsMessage};

// for ease of use
pub const DEFAULT_RNG: OsRng = OsRng;

pub(crate) type WsItem = Result<WsMessage, WsError>;

mod client;
pub mod error;
mod gateway;
mod state;

// Note: the handshake is built on top of WebSocket, but in principle it shouldn't be too difficult
// to remove that restriction, by just changing Sink<WsMessage> and Stream<Item = WsMessage> into
// AsyncWrite and AsyncRead and slightly adjusting the implementation. But right now
// we do not need to worry about that.

pub type DerivedSharedKey = Aes128Key;

pub(crate) trait RegistrationHandshake<S>:
    Future<Output = Result<(DerivedSharedKey, S), HandshakeError>>
{
}

pub async fn client_handshake<'a, S>(
    rng: &mut (impl RngCore + CryptoRng),
    ws_stream: S,
    identity: &'a identity::KeyPair,
    gateway_pubkey: identity::PublicKey,
    // TODO: perhaps include stream back in an error? like mpsc channels are doing?
) -> Result<(DerivedSharedKey, S), HandshakeError>
where
    S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin + 'a,
{
    // TODO: error map

    // ClientHandshake::new(rng, ws_stream, identity, gateway_pubkey).await
    todo!()
}

pub async fn gateway_handshake<'a, S>(
    rng: &mut (impl RngCore + CryptoRng),
    ws_stream: &'a mut S,
    identity: &'a identity::KeyPair,
    received_init_payload: Vec<u8>,
) -> Result<DerivedSharedKey, HandshakeError>
where
    S: Stream<Item = WsItem> + Sink<WsMessage> + Unpin + Send + 'a,
{
    // TODO: error map
    GatewayHandshake::new(rng, ws_stream, identity, received_init_payload).await
}

/*

Registration phase (Secure channel establishment using STS):
=== INIT CLIENT ==
1. Alice generates a random value x and computes g^x
2. Alice sends g^x to Gateway
== INIT GATE ==
3. Gateway generates a random value y and computes g^y
== MID GATE ==
4. Gateway computes the shared secret key k = KDF(gxy)  // As KDF we can use BLAKE3 instead of HKDF<SHA256> since BLAKE3 is ~15x faster than SHA256
5. Gateway concatenates (g^y, g^x), then signs them using its private key xG as sig = Sign(xG ,  (g^y, g^x)) and then encrypts the signature with k as c1 = AES(k, sig). This should be sent only once, so we can use IV of 0s.
6. Gateway sends g^y and c1 to Alice
== MID CLIENT ==
7. Alice computes the shared secret key k = KDF(gx)y
8. Using k Alice decrypts c1 and verifies Gateway’s signature using its public key yG
9. Alice concatenates (g^x, g^y), signs them using her private key xA as sigA = Sign(xA ,  (g^x, g^y)) and then encrypts using the signature with key k as c2 = AES(k, sigA).
10. Again, this should be sent only once, so we can use IV of 0s.
11. She sends c2 to the Gateway (no need to send g^x since it was already done in step 2.)
== FINAL GATE ==
12. Gateway decrypts c2 and verifies Alice's signature sigA using her public key yA
== FINAL CLIENT ==
13. So Gateway stores g^xA : k in a safe way in some dictionary etc.





C -> S:
CLIENT_PUB_KEY || G^x

S -> C
G^y || AES(k, SIG(PRIV_S, G^y || G^x))

C -> S
AES(k, SIG(PRIV_C, G^x || G^y))

S -> C
DONE


States:
INIT:
CLIENT:
- client priv, client pub
- gateway pub
- rand x
// - g^x
// - our asymmetric::encryption::KeyPair

GATEWAY:
[should] - client pub
- gateway priv, gateway pub
- [rand y]

C -> S: CLIENT_PUB_KEY || G^x
S -> C: G^y || AES(k, SIG(PRIV_S, G^y || G^x))

MID:
CLIENT:
- client priv, client pub
- gateway pub
- rand x
- g^y

GATEWAY:
- client pub_key
- g^x
- k
- g^y

C -> S: AES(k, SIG(PRIV_C, G^x || G^y))
S -> DONE


Role:
- our identity keypair:
- their pub identity
- their g^x
- ... k?


 */
