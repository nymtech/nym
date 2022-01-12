use crate::config::Config;
use crate::error::BackendError;
use validator_client::nymd::SigningNymdClient;
use validator_client::Client;

#[derive(Default)]
pub struct State {
  config: Config,
  signing_client: Option<Client<SigningNymdClient>>,
}

impl State {
  pub fn client(&self) -> Result<&Client<SigningNymdClient>, BackendError> {
    self
      .signing_client
      .as_ref()
      .ok_or(BackendError::ClientNotInitialized)
  }

  pub fn config(&self) -> Config {
    self.config.clone()
  }

  pub fn set_client(&mut self, signing_client: Client<SigningNymdClient>) {
    self.signing_client = Some(signing_client)
  }
}

#[macro_export]
macro_rules! client {
  ($state:ident) => {
    $state.read().await.client()?
  };
}

#[macro_export]
macro_rules! nymd_client {
  ($state:ident) => {
    $state.read().await.client()?.nymd
  };
}

#[macro_export]
macro_rules! api_client {
  ($state:ident) => {
    $state.read().await.client()?.validator_api
  };
}
