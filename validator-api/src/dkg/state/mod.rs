// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::{Epoch, NodeIndex};
use crypto::asymmetric::identity;
use dkg::{bte, Dealing};
use futures::lock::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

mod accessor;

pub(crate) use accessor::StateAccessor;

type IdentityBytes = [u8; identity::PUBLIC_KEY_LENGTH];

// TODO: some TryFrom impl to convert from encoded contract data
// note: each dealer is also a receiver which simplifies some logic significantly
#[derive(Debug)]
struct Dealer {
    node_index: NodeIndex,
    bte_public_key: bte::PublicKey,
    identity: identity::PublicKey,
    remote_address: SocketAddr,
}

#[derive(Debug, Clone)]
pub(crate) struct DkgState {
    inner: Arc<Mutex<DkgStateInner>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedDealing {
    epoch_id: u32,
    #[serde(with = "dealing_bytes")]
    dealing: Box<Dealing>,
    signature: identity::Signature,
}

mod dealing_bytes {
    use dkg::Dealing;
    use serde::de::Error as SerdeError;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_bytes::{ByteBuf as SerdeByteBuf, Bytes as SerdeBytes};

    pub fn serialize<S: Serializer>(val: &Dealing, serializer: S) -> Result<S::Ok, S::Error> {
        SerdeBytes::new(&val.to_bytes()).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Box<Dealing>, D::Error> {
        let bytes = <SerdeByteBuf>::deserialize(deserializer)?;
        let dealing = Dealing::try_from_bytes(bytes.as_ref()).map_err(SerdeError::custom)?;
        Ok(Box::new(dealing))
    }
}

#[derive(Debug)]
struct DkgStateInner {
    bte_decryption_key: bte::DecryptionKey,
    signing_key: identity::PublicKey,

    current_epoch: Epoch,

    expected_epoch_dealing_digests: HashMap<IdentityBytes, [u8; 32]>,
    current_epoch_dealers: HashMap<IdentityBytes, Dealer>,
    verified_epoch_dealings: HashMap<IdentityBytes, ReceivedDealing>,
    unconfirmed_dealings: HashMap<IdentityBytes, ReceivedDealing>,
}

impl DkgState {
    // some save/load action here
    pub(crate) async fn save(&self) {
        todo!()
    }

    pub(crate) async fn is_dealers_remote_address(&self, remote: SocketAddr) -> (bool, Epoch) {
        let guard = self.inner.lock().await;
        let epoch = guard.current_epoch;
        let dealers = &guard.current_epoch_dealers;

        (
            dealers
                .values()
                .any(|dealer| dealer.remote_address == remote),
            epoch,
        )
    }

    pub(crate) async fn current_epoch(&self) -> Epoch {
        self.inner.lock().await.current_epoch
    }

    pub(crate) async fn get_verified_dealing(
        &self,
        dealer: identity::PublicKey,
    ) -> Option<ReceivedDealing> {
        self.inner
            .lock()
            .await
            .verified_epoch_dealings
            .get(&dealer.to_bytes())
            .cloned()
    }
}
