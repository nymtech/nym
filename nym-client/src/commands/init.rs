use crate::built_info;
use crate::commands::override_config;
use crate::config::persistence::pathfinder::ClientPathfinder;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::identity::MixIdentityKeyPair;
use directory_client::presence::Topology;
use pemstore::pemstore::PemStore;
use sfw_provider_requests::AuthToken;
use sphinx::route::DestinationAddressBytes;
use topology::provider::Node;
use topology::NymTopology;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("init")
        .about("Initialise a Nym client. Do this first!")
        .arg(Arg::with_name("id")
            .long("id")
            .help("Id of the nym-mixnet-client we want to create config for.")
            .takes_value(true)
            .required(true)
        )
        .arg(Arg::with_name("provider")
            .long("provider")
            .help("Id of the provider we have preference to connect to. If left empty, a random provider will be chosen.")
            .takes_value(true)
        )
        .arg(Arg::with_name("directory")
                 .long("directory")
                 .help("Address of the directory server the client is getting topology from")
                 .takes_value(true),
        )
        .arg(Arg::with_name("socket-type")
            .long("socket-type")
            .help("Type of socket to use (TCP, WebSocket or None) in all subsequent runs")
            .takes_value(true)
        )
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .help("Port for the socket (if applicable) to listen on in all subsequent runs")
            .takes_value(true)
        )
}

async fn try_provider_registrations(
    providers: Vec<Node>,
    our_address: DestinationAddressBytes,
) -> Option<(String, AuthToken)> {
    // since the order of providers is non-deterministic we can just try to get a first 'working' provider
    for provider in providers {
        let provider_client = provider_client::ProviderClient::new(
            provider.client_listener,
            our_address.clone(),
            None,
        );
        let auth_token = provider_client.register().await;
        if let Ok(token) = auth_token {
            return Some((provider.pub_key, token));
        }
    }
    None
}

// in the long run this will be provider specific and only really applicable to a
// relatively small subset of all providers
async fn choose_provider(
    directory_server: String,
    our_address: DestinationAddressBytes,
) -> (String, AuthToken) {
    // TODO: once we change to graph topology this here will need to be updated!
    let topology = Topology::new(directory_server.clone());
    let version_filtered_topology = topology.filter_node_versions(
        built_info::PKG_VERSION,
        built_info::PKG_VERSION,
        built_info::PKG_VERSION,
    );
    // don't care about health of the networks as mixes can go up and down any time,
    // but DO care about providers
    let providers = version_filtered_topology.providers();

    // try to perform registration so that we wouldn't need to do it at startup
    // + at the same time we'll know if we can actually talk with that provider
    let registration_result = try_provider_registrations(providers, our_address).await;
    match registration_result {
        None => {
            // while technically there's no issue client-side, it will be impossible to execute
            // `nym-client run` as no provider is available so it might be best to not finalize
            // the init and rely on users trying to init another time?
            panic!(
                "Currently there are no valid providers available on the network ({}). \
                 Please try to run `init` again at later time or change your directory server",
                directory_server
            )
        }
        Some((provider_id, auth_token)) => (provider_id, auth_token),
    }
}

pub fn execute(matches: &ArgMatches) {
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap(); // required for now
    let mut config = crate::config::Config::new(id);

    config = override_config(config, matches);

    let mix_identity_keys = MixIdentityKeyPair::new();

    // if there is no provider chosen, get a random-ish one from the topology
    if config.get_provider_id().is_empty() {
        let our_address = mix_identity_keys.public_key().derive_address();
        // TODO: is there perhaps a way to make it work without having to spawn entire runtime?
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let (provider_id, auth_token) =
            rt.block_on(choose_provider(config.get_directory_server(), our_address));
        config = config
            .with_provider_id(provider_id)
            .with_provider_auth_token(auth_token);
    }

    let pathfinder = ClientPathfinder::new_from_config(&config);
    let pem_store = PemStore::new(pathfinder);
    pem_store
        .write_identity(mix_identity_keys)
        .expect("Failed to save identity keys");
    println!("Saved mixnet identity keypair");

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    println!("Saved configuration file to {:?}", config_save_location);

    println!(
        "Unless overridden in all `nym-client run` we will be talking to the following provider: {}...",
        config.get_provider_id(),
    );
    if config.get_provider_auth_token().is_some() {
        println!(
            "using optional AuthToken: {:?}",
            config.get_provider_auth_token().unwrap()
        )
    }
    println!("Client configuration completed.\n\n\n")
}
