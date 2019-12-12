use crate::banner;
use clap::ArgMatches;
use dirs;
use pem::{encode, Pem};
use std::fs::File;
use std::io::prelude::*;

pub fn execute(matches: &ArgMatches) {
    println!("{}", banner());
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap(); // required for now

    // don't unwrap it, pass it as it is, if it's None, choose a random
    let _provider_id = matches.value_of("provider");
    let _init_local = matches.is_present("local");

    let os_config_dir = dirs::config_dir().unwrap();
    let nym_client_config_dir = os_config_dir.join("nym").join("clients").join(id);

    println!("Writing keypairs to {:?}...", nym_client_config_dir);
    write_pem_files(nym_client_config_dir);

    println!("Client configuration completed.\n\n\n")
}

fn write_pem_files(nym_client_config_dir: std::path::PathBuf) {
    std::fs::create_dir_all(nym_client_config_dir.clone()).unwrap();

    let (private, public) = sphinx::crypto::keygen();
    write_pem_file(
        nym_client_config_dir.clone(),
        String::from("private.pem"),
        private.to_bytes().to_vec(),
        String::from("SPHINX CURVE25519 PRIVATE KEY"),
    );
    write_pem_file(
        nym_client_config_dir.clone(),
        String::from("public.pem"),
        public.to_bytes().to_vec(),
        String::from("SPHINX CURVE25519 PUBLIC KEY"),
    );
}

fn write_pem_file(
    nym_client_config_dir: std::path::PathBuf,
    filename: String,
    data: Vec<u8>,
    tag: String,
) {
    let pem = Pem {
        tag,
        contents: data,
    };
    let key = encode(&pem);

    let full_path = nym_client_config_dir.join(filename);
    let mut file = File::create(full_path).unwrap();
    file.write_all(key.as_bytes()).unwrap();
}
