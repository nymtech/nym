use crate::config::Config;
use validator_client::nymd::{NymdClient, SigningNymdClient};

#[derive(Default)]
pub struct State {
  config: Config,
  signing_client: Option<NymdClient<SigningNymdClient>>,
}

impl State {
  pub fn client(&self) -> Result<&NymdClient<SigningNymdClient>, String> {
    self.signing_client.as_ref().ok_or_else(|| {
      "Client has not been initialized yet, connect with mnemonic to initialize".to_string()
    })
  }

  pub fn config(&self) -> Config {
    self.config.clone()
  }

  pub fn set_client(&mut self, signing_client: NymdClient<SigningNymdClient>) {
    self.signing_client = Some(signing_client)
  }
}
