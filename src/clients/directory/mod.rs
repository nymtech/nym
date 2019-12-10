use sphinx::route::{Node as SphinxNode, Destination};
use crate::clients::directory::presence::models::Topology;
use reqwest::Error;

//use serde::Deserialize;

mod healthcheck;
mod metrics;
mod presence;

pub struct Config {
    pub base_url: String,
}

pub trait DirectoryClient {
    fn new(config : Config) -> Self;
    fn get_topology(&self) -> Result<Topology, reqwest::Error>;
//    fn send_provider_presence(&self) -> Result<ProviderPresenceResponse, reqwest::Error>;
}

pub struct Client {}

impl DirectoryClient for Client {
    fn new(config: Config) -> Client {
        let topology = retrieve_topology().unwrap();
        Client {}
    }

    fn get_topology(&self) -> Result<Topology, Error> {
        unimplemented!()
    }
}

fn retrieve_topology() -> Result<Topology, reqwest::Error> {
    let topology: Topology = reqwest::get("https://directory.nymtech.net/api/presence/topology")?
        .json()?;
    Ok(topology)
}


