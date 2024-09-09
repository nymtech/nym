// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash::bandwidth::issued::CURRENT_SERIALIZATION_REVISION;
use crate::Error;
use bincode::Options;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use zeroize::Zeroize;

pub mod keys;
pub mod signatures;

#[derive(Zeroize, Serialize, Deserialize)]
pub struct VersionSerialised<T: ?Sized> {
    pub data: Vec<u8>,
    pub revision: u8,

    // still wondering if there's any point in having the phantom in here
    #[zeroize(skip)]
    #[serde(skip)]
    _phantom: PhantomData<T>,
}

impl<T> VersionSerialised<T> {
    pub fn try_unpack(&self) -> Result<T, Error>
    where
        T: VersionedSerialise + DeserializeOwned,
    {
        T::try_unpack(&self.data, self.revision)
    }
}

pub trait VersionedSerialise {
    const CURRENT_SERIALISATION_REVISION: u8;

    fn current_serialization_revision(&self) -> u8 {
        CURRENT_SERIALIZATION_REVISION
    }

    // implicitly always uses current revision
    fn pack(&self) -> VersionSerialised<Self>
    where
        Self: Serialize,
    {
        let data = make_current_storable_bincode_serializer()
            .serialize(self)
            .expect("serialisation failure");

        VersionSerialised {
            data,
            revision: Self::CURRENT_SERIALISATION_REVISION,
            _phantom: Default::default(),
        }
    }

    fn try_unpack_current(b: &[u8]) -> Result<Self, Error>
    where
        Self: DeserializeOwned,
    {
        make_current_storable_bincode_serializer()
            .deserialize(b)
            .map_err(|source| Error::SerializationFailure {
                source,
                revision: Self::CURRENT_SERIALISATION_REVISION,
            })
    }

    // this is up to whoever implements the trait to provide function implementation,
    // as they might have to have different implementations per revision
    fn try_unpack(b: &[u8], revision: impl Into<Option<u8>>) -> Result<Self, Error>
    where
        Self: DeserializeOwned;
}

fn make_current_storable_bincode_serializer() -> impl bincode::Options {
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}
