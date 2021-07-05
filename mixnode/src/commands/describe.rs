use crate::commands::*;
use crate::config::Config;
use crate::node::node_description::NodeDescription;
use clap::{App, Arg, ArgMatches};
use colored::Colorize;
use config::NymConfig;
use std::io;
use std::io::Write;

pub fn command_args<'a, 'b>() -> App<'a, 'b> {
    App::new("describe")
        .about("Describe your mixnode and tell people why they should delegate stake to you")
        .arg(
            Arg::with_name(ID_ARG_NAME)
                .long(ID_ARG_NAME)
                .help("The id of the mixnode you want to describe")
                .takes_value(true)
                .required(true),
        )
}

pub fn execute(matches: &ArgMatches) {
    // figure out which node the user is describing
    let id = matches
        .value_of(ID_ARG_NAME)
        .expect("Please provide the id of your mixnode");

    // ensure that the mixnode has in fact been initialized
    match Config::load_from_file(Some(id)) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})", id, err);
            return;
        }
    };

    // get input from the user
    print!("name: ");
    io::stdout().flush().unwrap();
    let mut name_buf = String::new();
    io::stdin().read_line(&mut name_buf).unwrap();
    let name = name_buf.trim().to_string();

    print!("description: ");
    io::stdout().flush().unwrap();
    let mut desc_buf = String::new();
    io::stdin().read_line(&mut desc_buf).unwrap();
    let description = desc_buf.trim().to_string();

    let example_url = "https://mixnode.yourdomain.com".bright_cyan();
    let example_location = "City: London, Country: UK";

    print!("link, e.g. {}: ", example_url);
    io::stdout().flush().unwrap();
    let mut link_buf = String::new();
    io::stdin().read_line(&mut link_buf).unwrap();
    let link = link_buf.trim().to_string();

    print!("location, e.g. {}: ", example_location);
    io::stdout().flush().unwrap();
    let mut location_buf = String::new();
    io::stdin().read_line(&mut location_buf).unwrap();
    let location = location_buf.trim().to_string();

    let node_description = NodeDescription {
        name,
        description,
        link,
        location,
    };

    // save the struct
    NodeDescription::save_to_file(
        &node_description,
        Config::default_config_directory(Some(id)),
    )
    .unwrap()
}
