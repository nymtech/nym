use clap::ArgMatches;

pub fn execute(matches: &ArgMatches) {
    let port = match matches.value_of("port").unwrap().parse::<u16>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid port value provided - {:?}", err),
    };

    println!("On the following port: {:?}", port);
}
