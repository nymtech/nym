use std::process;
use std::time::Duration;

use clap::{App, Arg, ArgMatches, SubCommand};
use tokio::runtime::Runtime;
use tokio::time::{interval_at, Instant};

use crate::clients::directory;
use crate::clients::directory::requests::health_check_get::HealthCheckRequester;
use crate::clients::directory::DirectoryClient;

mod clients;

const TCP_SOCKET_TYPE: &str = "tcp";
const WEBSOCKET_SOCKET_TYPE: &str = "websocket";

fn execute(matches: ArgMatches) -> Result<(), String> {
    match matches.subcommand() {
        ("init", Some(m)) => Ok(init(m)),
        ("run", Some(m)) => Ok(run(m)),
        ("socket", Some(m)) => Ok(socket(m)),

        _ => Err(String::from("Unknown command")),
    }
}

fn init(matches: &ArgMatches) {
    println!("Running client init!");

    // don't unwrap it, pass it as it is, if it's None, choose a random
    let provider_id = matches.value_of("providerID");
    let init_local = matches.is_present("local");

    println!(
        "client init goes here with providerID: {:?} and running locally: {:?}",
        provider_id, init_local
    );
}

fn run(matches: &ArgMatches) {
    let custom_cfg = matches.value_of("customCfg");
    println!(
        "Going to start client with custom config of: {:?}",
        custom_cfg
    );

    // Create the runtime, probably later move it to Client struct itself?
    let mut rt = Runtime::new().unwrap();

    // Spawn the root task
    rt.block_on(async {
        let start = Instant::now() + Duration::from_nanos(1000);
        let mut interval = interval_at(start, Duration::from_millis(5000));
        let mut i: usize = 0;
        loop {
            interval.tick().await;
            let message = format!("Hello, Sphinx {}", i).as_bytes().to_vec();

            // set up the route
            let directory_config = directory::Config {
                base_url: "https://directory.nymtech.net".to_string(),
            };
            let directory = clients::directory::Client::new(directory_config);

            // make sure the Directory server is in fact running, panic if not
            directory
                .health_check
                .get()
                .expect("Directory health check failed, is the Directory server running?");

            //            let route = directory.get_mixes();
            //            let destination = directory.get_destination();
            let delays = sphinx::header::delays::generate(2);

            //            println!("delays: {:?}", delays);
            // build the packet
            //            let packet = sphinx::SphinxPacket::new(message, &route[..], &destination, &delays).unwrap();
            //
            //            // send to mixnet
            //            let mix_client = MixClient::new();
            //            let result = mix_client.send(packet, route.first().unwrap()).await;
            //            println!("packet sent:  {:?}", i);
            //            i += 1;
        }
    })
}

fn socket(matches: &ArgMatches) {
    let custom_cfg = matches.value_of("customCfg");
    let socket_type = match matches.value_of("socketType").unwrap() {
        TCP_SOCKET_TYPE => TCP_SOCKET_TYPE,
        WEBSOCKET_SOCKET_TYPE => WEBSOCKET_SOCKET_TYPE,
        other => panic!("Invalid socket type provided - {}", other),
    };
    let port = match matches.value_of("port").unwrap().parse::<u16>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid port value provided - {:?}", err),
    };

    println!(
        "Going to start socket client with custom config of: {:?}",
        custom_cfg
    );
    println!("Using the following socket type: {:?}", socket_type);
    println!("On the following port: {:?}", port);
}

// TODO: perhaps more subcommands and/or args to distinguish between coco client and mix client
fn main() {
    let arg_matches = App::new("Nym Client")
        .version("0.1.0")
        .author("Nymtech")
        .about("Implementation of the Nym Client")
        .subcommand(
            SubCommand::with_name("init")
                .about("Initialise a Nym client")
                .arg(Arg::with_name("providerID")
                    .short("pid")
                    .long("providerID")
                    .help("Id of the provider we have preference to connect to. If left empty, a random provider will be chosen")
                    .takes_value(true)
                )
                .arg(Arg::with_name("local")
                    .short("loc")
                    .long("local")
                    .help("Flag to indicate whether the client is expected to run on the local deployment")
                )
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Run a persistent Nym client process")
                .arg(
                    Arg::with_name("customCfg")
                        .short("cfg")
                        .long("customCfg")
                        .help("Path to custom configuration file of the client")
                        .takes_value(true)
                )
        )
        .subcommand(
            SubCommand::with_name("socket")
                .about("Run a background Nym client listening on a specified socket")
                .arg(
                    Arg::with_name("customCfg")
                        .short("cfg")
                        .long("customCfg")
                        .help("Path to custom configuration file of the client")
                        .takes_value(true)
                )
                .arg(
                    Arg::with_name("socketType")
                        .short("s")
                        .long("socketType")
                        .help("Type of the socket we want to run on (tcp / websocket)")
                        .takes_value(true)
                        .required(true)
                )
                .arg(
                    Arg::with_name("port")
                        .short("p")
                        .long("port")
                        .help("Port to listen on")
                        .takes_value(true)
                        .required(true),
                )
        )
        .get_matches();

    if let Err(e) = execute(arg_matches) {
        println!("Application error: {}", e);
        process::exit(1);
    }
}
