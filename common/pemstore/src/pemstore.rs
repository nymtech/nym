use crate::pathfinder::PathFinder;
use crypto::identity::MixIdentityKeyPair;
use crypto::PemStorableKey;
use crypto::{encryption, PemStorableKeyPair};
use log::info;
use pem::{encode, parse, Pem};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;

pub struct PemStore {
    #[allow(dead_code)]
    config_dir: PathBuf,
    private_mix_key_file: PathBuf,
    public_mix_key_file: PathBuf,
}

impl PemStore {
    pub fn new<P: PathFinder>(pathfinder: P) -> PemStore {
        PemStore {
            config_dir: pathfinder.config_dir(),
            private_mix_key_file: pathfinder.private_identity_key(),
            public_mix_key_file: pathfinder.public_identity_key(),
        }
    }

    pub fn read_keypair<T: PemStorableKeyPair>(&self) -> io::Result<T> {
        let private_pem = self.read_pem_file(self.private_mix_key_file.clone())?;
        let public_pem = self.read_pem_file(self.public_mix_key_file.clone())?;

        let key_pair = T::from_bytes(&private_pem.contents, &public_pem.contents);

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

    pub fn read_encryption(&self) -> io::Result<encryption::KeyPair> {
        self.read_keypair()
    }

    pub fn read_identity(&self) -> io::Result<MixIdentityKeyPair> {
        self.read_keypair()
    }

    fn read_pem_file(&self, filepath: PathBuf) -> io::Result<Pem> {
        let mut pem_bytes = File::open(filepath)?;
        let mut buf = Vec::new();
        pem_bytes.read_to_end(&mut buf)?;
        parse(&buf).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn write_keypair(&self, key_pair: impl PemStorableKeyPair) -> io::Result<()> {
        let private_key = key_pair.private_key();
        let public_key = key_pair.public_key();

        self.write_pem_file(
            self.private_mix_key_file.clone(),
            private_key.to_bytes(),
            private_key.pem_type(),
        )?;
        info!(
            "Written private key to {:?}",
            self.private_mix_key_file.clone()
        );
        self.write_pem_file(
            self.public_mix_key_file.clone(),
            public_key.to_bytes(),
            public_key.pem_type(),
        )?;
        info!(
            "Written public key to {:?}",
            self.public_mix_key_file.clone()
        );
        Ok(())
    }

    // This should be refactored and made more generic for when we have other kinds of
    // KeyPairs that we want to persist (e.g. validator keypairs, or keys for
    // signing vs encryption). However, for the moment, it does the job.
    pub fn write_identity(&self, key_pair: MixIdentityKeyPair) -> io::Result<()> {
        self.write_keypair(key_pair)
    }

    pub fn write_encryption_keys(&self, key_pair: encryption::KeyPair) -> io::Result<()> {
        self.write_keypair(key_pair)
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
