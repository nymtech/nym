use clap::ArgMatches;
use pem::{encode, Pem};
use std::fs::File;
use std::io::prelude::*;

pub fn execute(matches: &ArgMatches) {
    println!("Initialising client...");

    // don't unwrap it, pass it as it is, if it's None, choose a random
    let id = matches.value_of("id");
    let provider_id = matches.value_of("provider");
    let init_local = matches.is_present("local");

    println!(
        "id: {:?}, provider: {:?}, local: {:?}",
        id, provider_id, init_local
    );

    write_pem_files();
}

fn write_pem_files() {
    let key_directory = String::from("/home/dave/.nym/clients/foomp/config");
    std::fs::create_dir_all(key_directory.clone()).unwrap();

    let (private, public) = sphinx::crypto::keygen();
    write_pem_file(
        key_directory.clone(),
        String::from("secret.pem"),
        private.to_bytes().to_vec(),
        String::from("SPHINX CURVE25519 PRIVATE KEY"),
    );
    write_pem_file(
        key_directory.clone(),
        String::from("public.pem"),
        public.to_bytes().to_vec(),
        String::from("SPHINX CURVE25519 PUBLIC KEY"),
    );
}

fn write_pem_file(directory: String, filename: String, data: Vec<u8>, tag: String) {
    let pem = Pem {
        tag,
        contents: data,
    };
    let key = encode(&pem);

    let full_path = directory + "/" + &filename;
    let mut file = File::create(full_path).unwrap();
    file.write_all(key.as_bytes()).unwrap();
}
