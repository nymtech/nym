use crate::clients::NymClient;
use clap::ArgMatches;

pub fn execute(matches: &ArgMatches) {
    let is_local = matches.is_present("local");
    println!("Starting client, local: {:?}", is_local);

    // TODO: to be taken from config or something
    let my_address = [42u8; 32];
    let is_local = true;
    let client = NymClient::new(my_address, is_local);
    client.start().unwrap();
}
