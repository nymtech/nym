use sphinx::route::{Node as SphinxNode, Destination};
use crate::clients::directory::presence::models::Topology;
use reqwest::{Error};
use crate::clients::directory::healthcheck::requests::{Requester, HealthCheckRequester};

//use serde::Deserialize;

pub mod healthcheck;
mod metrics;
mod presence;

pub struct Config {
    pub base_url: String,
}

pub trait DirectoryClient {
    fn new(config : Config) -> Self;
    fn health_check(&self) -> Result<reqwest::Response, reqwest::Error>;
    fn get_topology(&self) -> Result<Topology, reqwest::Error>;
//    fn send_provider_presence(&self) -> Result<ProviderPresenceResponse, reqwest::Error>;
}

pub struct Client {
    pub health_check: Requester,
}

impl DirectoryClient for Client {
    fn new(config: Config) -> Client {
        let topology = retrieve_topology().unwrap();
        let hcr: Requester = Requester::new(config.base_url);
        Client {
            health_check: hcr,
        }
    }

    fn health_check(&self) -> Result<reqwest::Response, reqwest::Error> {
        self.health_check.make_request()
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


