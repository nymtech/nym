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

use crate::pathfinder::PathFinder;
use crypto::asymmetric::{encryption, identity};
use crypto::{PemStorableKey, PemStorableKeyPair};
use log::info;
use pem::{encode, parse, Pem};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;

pub struct PemStore<P> {
    pathfinder: P,
}

enum KeyType {
    Identity,
    Encryption,
}

impl<P> PemStore<P> {
    pub fn new(pathfinder: P) -> Self {
        PemStore { pathfinder }
    }

    fn get_keypair_paths(&self, key_type: KeyType) -> (PathBuf, PathBuf)
    where
        P: PathFinder,
    {
        match key_type {
            KeyType::Identity => (
                self.pathfinder.private_identity_key(),
                self.pathfinder.public_identity_key(),
            ),
            KeyType::Encryption => (
                self.pathfinder
                    .private_encryption_key()
                    .expect("tried to write encryption keypair while no path was specified"),
                self.pathfinder
                    .public_encryption_key()
                    .expect("tried to write encryption keypair while no path was specified"),
            ),
        }
    }

    fn read_keypair<T>(&self, key_type: KeyType) -> io::Result<T>
    where
        P: PathFinder,
        T: PemStorableKeyPair,
    {
        let (private_key_path, public_key_path) = self.get_keypair_paths(key_type);

        let private_pem = self.read_pem_file(private_key_path)?;
        let public_pem = self.read_pem_file(public_key_path)?;

        let key_pair = match T::from_bytes(&private_pem.contents, &public_pem.contents) {
            Ok(key_pair) => key_pair,
            Err(err) => return Err(io::Error::new(io::ErrorKind::InvalidData, err.to_string())),
        };

        if key_pair.private_key().pem_type() != private_pem.tag {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "unexpected private key pem tag",
            ));
        }

        if key_pair.public_key().pem_type() != public_pem.tag {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "unexpected public key pem tag",
            ));
        }

        Ok(key_pair)
    }

    pub fn read_encryption(&self) -> io::Result<encryption::KeyPair>
    where
        P: PathFinder,
    {
        self.read_keypair(KeyType::Encryption)
    }

    pub fn read_identity(&self) -> io::Result<identity::KeyPair>
    where
        P: PathFinder,
    {
        self.read_keypair(KeyType::Identity)
    }

    fn read_pem_file(&self, filepath: PathBuf) -> io::Result<Pem> {
        let mut pem_bytes = File::open(filepath)?;
        let mut buf = Vec::new();
        pem_bytes.read_to_end(&mut buf)?;
        parse(&buf).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn write_keypair<T>(&self, key_pair: &T, key_type: KeyType) -> io::Result<()>
    where
        P: PathFinder,
        T: PemStorableKeyPair,
    {
        let private_key = key_pair.private_key();
        let public_key = key_pair.public_key();

        let (private_key_path, public_key_path) = self.get_keypair_paths(key_type);

        self.write_pem_file(
            private_key_path.clone(),
            private_key.to_bytes(),
            private_key.pem_type(),
        )?;
        info!("Written private key to {:?}", private_key_path);
        self.write_pem_file(
            public_key_path.clone(),
            public_key.to_bytes(),
            public_key.pem_type(),
        )?;
        info!("Written public key to {:?}", public_key_path);
        Ok(())
    }

    // This should be refactored and made more generic for when we have other kinds of
    // KeyPairs that we want to persist (e.g. validator keypairs, or keys for
    // signing vs encryption). However, for the moment, it does the job.
    pub fn write_identity(&self, key_pair: &identity::KeyPair) -> io::Result<()>
    where
        P: PathFinder,
    {
        self.write_keypair(key_pair, KeyType::Identity)
    }

    pub fn write_encryption_keys(&self, key_pair: &encryption::KeyPair) -> io::Result<()>
    where
        P: PathFinder,
    {
        self.write_keypair(key_pair, KeyType::Encryption)
    }

    fn write_pem_file(&self, filepath: PathBuf, data: Vec<u8>, tag: String) -> io::Result<()> {
        // ensure the whole directory structure exists
        if let Some(parent_dir) = filepath.parent() {
            std::fs::create_dir_all(parent_dir)?;
        }
        let pem = Pem {
            tag,
            contents: data,
        };
        let key = encode(&pem);

        let mut file = File::create(filepath)?;
        file.write_all(key.as_bytes())?;

        Ok(())
    }
}
