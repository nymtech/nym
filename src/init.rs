use clap::ArgMatches;

pub fn init(matches: &ArgMatches) {
    println!("Running client init!");

    // don't unwrap it, pass it as it is, if it's None, choose a random
    let provider_id = matches.value_of("providerID");
    let init_local = matches.is_present("local");

    println!(
        "client init goes here with providerID: {:?} and running locally: {:?}",
        provider_id, init_local
    );
}
