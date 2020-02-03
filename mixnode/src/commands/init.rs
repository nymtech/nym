use clap::{App, Arg};

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("init").about("Initialise the mixnode")
}
