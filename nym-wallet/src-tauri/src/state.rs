use crate::config::Config;
use crate::error::BackendError;
use validator_client::nymd::{NymdClient, SigningNymdClient};

#[derive(Default)]
pub struct State {
  config: Config,
  signing_client: Option<NymdClient<SigningNymdClient>>,
}

impl State {
  pub fn client(&self) -> Result<&NymdClient<SigningNymdClient>, BackendError> {
    self
      .signing_client
      .as_ref()
      .ok_or(BackendError::ClientNotInitialized)
  }

  pub fn config(&self) -> Config {
    self.config.clone()
  }

  pub fn set_client(&mut self, signing_client: NymdClient<SigningNymdClient>) {
    self.signing_client = Some(signing_client)
  }
}

#[macro_export]
macro_rules! client {
  ($state:ident) => {
    $state.read().await.client()?
  };
}
