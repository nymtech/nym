use sphinx::route::{Node as SphinxNode, Destination};
use reqwest::{Error};
use crate::clients::directory::requests::health_check_get::{Request, HealthCheckRequester};
use crate::clients::directory::presence::Topology;

//use serde::Deserialize;

pub mod requests;
mod presence;

pub struct Config {
    pub base_url: String,
}

pub trait DirectoryClient {
    fn new(config : Config) -> Self;
    fn get_topology(&self) -> Result<Topology, reqwest::Error>;
//    fn send_provider_presence(&self) -> Result<ProviderPresenceResponse, reqwest::Error>;
}

pub struct Client {
    pub health_check: Request,
}

impl DirectoryClient for Client {
    fn new(config: Config) -> Client {
        let topology = retrieve_topology().unwrap();
        let hcr: Request = Request::new(config.base_url);
        Client {
            health_check: hcr,
        }
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


