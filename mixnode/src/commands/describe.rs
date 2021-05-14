use crate::config::Config;
use crate::node::node_description::NodeDescription;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use rustyline::Editor;

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
    let mut rl = Editor::<()>::new();
    let name = rl.readline("node name: ").unwrap();
    let description = rl.readline("node description: ").unwrap();
    let link = rl.readline("link (start with 'http://'': ").unwrap();

    let node_description = NodeDescription {
        name,
        description,
        link,
    };

    // save the struct
    let config_path = Config::default_config_directory(id);
    NodeDescription::save_to_file(&node_description, config_path).unwrap();
}
