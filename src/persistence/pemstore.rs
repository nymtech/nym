use crate::identity::mixnet::KeyPair;
use crate::persistence::pathfinder::Pathfinder;
use pem::{encode, parse, Pem};
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

pub struct PemStore {
    config_dir: PathBuf,
    private_mix_key: PathBuf,
    public_mix_key: PathBuf,
}

impl PemStore {
    pub fn new(pathfinder: Pathfinder) -> PemStore {
        PemStore {
            config_dir: pathfinder.config_dir,
            private_mix_key: pathfinder.private_mix_key,
            public_mix_key: pathfinder.public_mix_key,
        }
    }

    pub fn read(&self) -> KeyPair {
        let private = self.read_file(self.private_mix_key.clone());
        let public = self.read_file(self.public_mix_key.clone());

        KeyPair::from_bytes(private, public)
    }

    pub fn read_file(&self, filepath: PathBuf) -> Vec<u8> {
        let mut pem_bytes = File::open(filepath).unwrap();
        let mut buf = Vec::new();
        pem_bytes.read_to_end(&mut buf).unwrap();
        let pem = parse(&buf).unwrap();
        pem.contents
    }
    // This should be refactored and made more generic for when we have other kinds of
    // KeyPairs that we want to persist (e.g. validator keypairs, or keys for
    // signing vs encryption). However, for the moment, it does the job.
    pub fn write(&self, key_pair: KeyPair) {
        std::fs::create_dir_all(self.config_dir.clone()).unwrap();

        self.write_pem_file(
            self.private_mix_key.clone(),
            key_pair.private_bytes(),
            String::from("SPHINX CURVE25519 PRIVATE KEY"),
        );
        self.write_pem_file(
            self.public_mix_key.clone(),
            key_pair.public_bytes(),
            String::from("SPHINX CURVE25519 PUBLIC KEY"),
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
