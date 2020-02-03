use clap::{App, Arg, ArgMatches};
use config::NymConfig;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("init").about("Initialise the mixnode").arg(
        Arg::with_name("id")
            .long("id")
            .help("Id of the nym-mixnode we want to create config for.")
            .takes_value(true)
            .required(true),
    )
}

pub fn execute(matches: &ArgMatches) {
    println!("Initialising mixnode...");

    let id = matches.value_of("id").unwrap(); // required for now
    let mut config = crate::config::Config::new(id);

    // overriding config defaults here

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    println!("Saved configuration file to {:?}", config_save_location);

    println!("Mixnode configuration completed.\n\n\n")
}
