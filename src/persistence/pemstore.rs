use crate::identity::mixnet::KeyPair;
use pem::{encode, Pem};
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

pub struct PemStore {}

impl PemStore {
    pub fn new() -> PemStore {
        PemStore {}
    }
    pub fn write(&self, key_pair: KeyPair, path: PathBuf) {
        std::fs::create_dir_all(path.clone()).unwrap();

        self.write_pem_file(
            path.clone(),
            String::from("private.pem"),
            key_pair.private_bytes(),
            String::from("SPHINX CURVE25519 PRIVATE KEY"),
        );
        self.write_pem_file(
            path.clone(),
            String::from("public.pem"),
            key_pair.public_bytes(),
            String::from("SPHINX CURVE25519 PUBLIC KEY"),
        );
    }

    fn write_pem_file(&self, path: PathBuf, filename: String, data: Vec<u8>, tag: String) {
        let pem = Pem {
            tag,
            contents: data,
        };
        let key = encode(&pem);

        let full_path = path.join(filename);
        let mut file = File::create(full_path).unwrap();
        file.write_all(key.as_bytes()).unwrap();
    }
}
