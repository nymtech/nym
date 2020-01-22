use crate::pathfinder::PathFinder;
use pem::{encode, parse, Pem};
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

#[allow(dead_code)]
pub fn read_mix_encryption_keypair_from_disk(_id: String) -> crypto::encryption::x25519::KeyPair {
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

    pub fn read_identity<IDPair, Priv, Pub>(&self) -> IDPair
    where
        IDPair: crypto::identity::MixnetIdentityKeyPair<Priv, Pub>,
        Priv: crypto::identity::MixnetIdentityPrivateKey,
        Pub: crypto::identity::MixnetIdentityPublicKey,
    {
        let private_pem = self.read_pem_file(self.private_mix_key.clone());
        let public_pem = self.read_pem_file(self.public_mix_key.clone());

        let key_pair = IDPair::from_bytes(&private_pem.contents, &public_pem.contents);

        assert_eq!(key_pair.private_key().pem_type(), private_pem.tag);
        assert_eq!(key_pair.public_key().pem_type(), public_pem.tag);

        key_pair
    }

    fn read_pem_file(&self, filepath: PathBuf) -> Pem {
        let mut pem_bytes = File::open(filepath).expect("Could not open stored keys from disk.");
        let mut buf = Vec::new();
        pem_bytes
            .read_to_end(&mut buf)
            .expect("PEM bytes reading failed.");
        let pem = parse(&buf).expect("PEM parsing failed while reading keys");

        pem
    }
    // This should be refactored and made more generic for when we have other kinds of
    // KeyPairs that we want to persist (e.g. validator keypairs, or keys for
    // signing vs encryption). However, for the moment, it does the job.
    pub fn write_identity<IDPair, Priv, Pub>(&self, key_pair: IDPair)
    where
        IDPair: crypto::identity::MixnetIdentityKeyPair<Priv, Pub>,
        Priv: crypto::identity::MixnetIdentityPrivateKey,
        Pub: crypto::identity::MixnetIdentityPublicKey,
    {
        std::fs::create_dir_all(self.config_dir.clone()).unwrap();

        let private_key = key_pair.private_key();
        let public_key = key_pair.public_key();
        self.write_pem_file(
            self.private_mix_key.clone(),
            private_key.to_bytes(),
            private_key.pem_type(),
        );
        self.write_pem_file(
            self.public_mix_key.clone(),
            public_key.to_bytes(),
            public_key.pem_type(),
        );
    }

    fn write_pem_file(&self, filepath: PathBuf, data: Vec<u8>, tag: String) {
        let pem = Pem {
            tag,
            contents: data,
        };
        let key = encode(&pem);

        let mut file = File::create(filepath).unwrap();
        file.write_all(key.as_bytes()).unwrap();
    }
}
