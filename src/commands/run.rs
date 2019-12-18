use crate::banner;
use crate::clients::NymClient;
use crate::persistence::pemstore;
use clap::ArgMatches;

pub fn execute(matches: &ArgMatches) {
    println!("{}", banner());
    panic!("For time being this command is deprecated! Please use 'websocket' instead");
    //
    //    let is_local = matches.is_present("local");
    //    let id = matches.value_of("id").unwrap().to_string();
    //    println!("Starting client...");
    //
    //    let keypair = pemstore::read_keypair_from_disk(id);
    //    let client = NymClient::new(keypair.public_bytes(), is_local);
    //    client.start("127.0.0.1:9000".parse().unwrap()).unwrap();
}
