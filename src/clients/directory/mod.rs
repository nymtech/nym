use crate::clients::directory::presence::Topology;
use crate::clients::directory::requests::health_check_get::{
    HealthCheckRequester, Request as HealthCheckRequest,
};
use crate::clients::directory::requests::presence_topology_get::{
    PresenceTopologyGetRequester, Request as PresenceTopologyRequest,
};
use reqwest::Error;
use sphinx::route::Destination;

mod presence;
pub mod requests;

pub struct Config {
    pub base_url: String,
}

pub trait DirectoryClient {
    fn new(config: Config) -> Self;
}

pub struct Client {
    pub health_check: HealthCheckRequest,
    pub presence_topology: PresenceTopologyRequest,
}

impl DirectoryClient for Client {
    fn new(config: Config) -> Client {
        let health_check: HealthCheckRequest = HealthCheckRequest::new(config.base_url.clone());
        let presence_topology: PresenceTopologyRequest =
            PresenceTopologyRequest::new(config.base_url.clone());
        Client {
            health_check,
            presence_topology,
        }
    }
}
