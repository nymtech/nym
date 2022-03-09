use crate::config::Config;
use crate::error::BackendError;
use crate::network::Network;
use validator_client::nymd::SigningNymdClient;
use validator_client::Client;

use std::collections::HashMap;

#[derive(Default)]
pub struct State {
  config: Config,
  signing_clients: HashMap<Network, Client<SigningNymdClient>>,
  current_network: Network,
}

impl State {
  pub fn client(&self, network: Network) -> Result<&Client<SigningNymdClient>, BackendError> {
    self
      .signing_clients
      .get(&network)
      .ok_or(BackendError::ClientNotInitialized)
  }

  pub fn current_client(&self) -> Result<&Client<SigningNymdClient>, BackendError> {
    self
      .signing_clients
      .get(&self.current_network)
      .ok_or(BackendError::ClientNotInitialized)
  }

  pub fn config(&self) -> Config {
    self.config.clone()
  }

  pub fn add_client(&mut self, network: Network, client: Client<SigningNymdClient>) {
    self.signing_clients.insert(network, client);
  }

  pub fn set_network(&mut self, network: Network) {
    self.current_network = network;
  }

  pub fn current_network(&self) -> Network {
    self.current_network
  }

  pub fn logout(&mut self) {
    self.signing_clients = HashMap::new();
  }
}

#[macro_export]
macro_rules! client {
  ($state:ident) => {
    $state.read().await.current_client()?
  };
}

#[macro_export]
macro_rules! nymd_client {
  ($state:ident) => {
    $state.read().await.current_client()?.nymd
  };
}

#[macro_export]
macro_rules! api_client {
  ($state:ident) => {
    $state.read().await.current_client()?.validator_api
  };
}
