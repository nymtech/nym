use clap::ArgMatches;

pub fn socket(matches: &ArgMatches) {
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
