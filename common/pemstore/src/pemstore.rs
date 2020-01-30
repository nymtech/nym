use crate::pathfinder::PathFinder;
use crypto::identity::MixIdentityKeyPair;
use crypto::PemStorable;
use pem::{encode, parse, Pem};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;

#[allow(dead_code)]
pub fn read_mix_encryption_keypair_from_disk(_id: String) -> crypto::encryption::KeyPair {
    unimplemented!()
}

pub struct PemStore {
    config_dir: PathBuf,
    private_mix_key: PathBuf,
    public_mix_key: PathBuf,
}

impl PemStore {
    pub fn new<P: PathFinder>(pathfinder: P) -> PemStore {
        PemStore {
            config_dir: pathfinder.config_dir(),
            private_mix_key: pathfinder.private_identity_key(),
            public_mix_key: pathfinder.public_identity_key(),
        }
    }

    pub fn read_identity(&self) -> io::Result<MixIdentityKeyPair> {
        let private_pem = self.read_pem_file(self.private_mix_key.clone())?;
        let public_pem = self.read_pem_file(self.public_mix_key.clone())?;

        let key_pair = MixIdentityKeyPair::from_bytes(&private_pem.contents, &public_pem.contents);

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

    fn read_pem_file(&self, filepath: PathBuf) -> io::Result<Pem> {
        let mut pem_bytes = File::open(filepath)?;
        let mut buf = Vec::new();
        pem_bytes.read_to_end(&mut buf)?;
        parse(&buf).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
    // This should be refactored and made more generic for when we have other kinds of
    // KeyPairs that we want to persist (e.g. validator keypairs, or keys for
    // signing vs encryption). However, for the moment, it does the job.
    pub fn write_identity(&self, key_pair: MixIdentityKeyPair) -> io::Result<()> {
        std::fs::create_dir_all(self.config_dir.clone())?;

        let private_key = key_pair.private_key();
        let public_key = key_pair.public_key();
        self.write_pem_file(
            self.private_mix_key.clone(),
            private_key.to_bytes(),
            private_key.pem_type(),
        )?;
        self.write_pem_file(
            self.public_mix_key.clone(),
            public_key.to_bytes(),
            public_key.pem_type(),
        )?;
        Ok(())
    }

    fn write_pem_file(&self, filepath: PathBuf, data: Vec<u8>, tag: String) -> io::Result<()> {
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
