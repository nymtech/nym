use crate::clients::directory::presence::Topology;
use crate::clients::directory::requests::health_check_get::{HealthCheckRequester, Request};
use reqwest::Error;

mod presence;
pub mod requests;

pub struct Config {
    pub base_url: String,
}

pub trait DirectoryClient {
    fn new(config: Config) -> Self;
    fn get_topology(&self) -> Result<Topology, reqwest::Error>;
    //    fn send_provider_presence(&self) -> Result<ProviderPresenceResponse, reqwest::Error>;
}

pub struct Client {
    pub health_check: Request,
}

impl DirectoryClient for Client {
    fn new(config: Config) -> Client {
        let hcr: Request = Request::new(config.base_url);
        Client { health_check: hcr }
    }

    fn get_topology(&self) -> Result<Topology, Error> {
        unimplemented!()
    }
}
