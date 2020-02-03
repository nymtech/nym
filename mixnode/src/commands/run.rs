use clap::{App, Arg};

pub fn command_args<'a, 'b>() -> App<'a, 'b> {
    App::new("run")
        .about("Starts the mixnode")
        .arg(
            Arg::with_name("host")
                .long("host")
                .help("The custom host on which the mixnode will be running")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .help("The port on which the mixnode will be listening")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("layer")
                .long("layer")
                .help("The mixnet layer of this particular node")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("announce_host")
                .long("announce-host")
                .help("The host that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("announce_port")
                .long("announce-port")
                .help("The port that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("directory")
                .long("directory")
                .help("Address of the directory server the node is sending presence and metrics to")
                .takes_value(true),
        )
}
