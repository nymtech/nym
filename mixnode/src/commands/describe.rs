use crate::config::Config;
use crate::node::node_description::NodeDescription;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use std::io;
use std::io::Write;

pub fn command_args<'a, 'b>() -> App<'a, 'b> {
    App::new("describe")
        .about("Describe your mixnode and tell people why they should delegate stake to you")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("The id of the mixnode you want to describe")
                .takes_value(true)
                .required(true),
        )
}

pub fn execute(matches: &ArgMatches) {
    // figure out which node the user is describing
    let id = matches.value_of("id").unwrap();

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

    print!("link: ");
    io::stdout().flush().unwrap();
    let mut link_buf = String::new();
    io::stdin().read_line(&mut link_buf).unwrap();
    let link = link_buf.trim().to_string();

    let node_description = NodeDescription {
        name,
        description,
        link,
    };

    // save the struct
    let config_path = Config::default_config_directory(id);
    NodeDescription::save_to_file(&node_description, config_path).unwrap();
}
