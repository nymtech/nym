use crate::config::Config;
use crate::format_err;
use coconut_interface::{
  self, Attribute, Parameters, Signature, VerificationKey,
};
use validator_client::nymd::{NymdClient, SigningNymdClient};

#[derive(Default)]
pub struct State {
  config: Config,
  signing_client: Option<NymdClient<SigningNymdClient>>,
  // Coconut stuff
  pub signatures: Vec<Signature>,
  n_attributes: u32,
  params: Option<Parameters>,
  public_attributes: Vec<Attribute>,
  private_attributes: Vec<Attribute>,
  pub aggregated_verification_key: Option<VerificationKey>,
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

  pub fn params(&self) -> Result<&Parameters, String> {
    self
      .params
      .as_ref()
      .ok_or_else(|| format_err!("Parameters are not set!"))
  }

  pub fn private_attributes(&self) -> Vec<Attribute> {
    self.private_attributes.clone()
  }

  pub fn public_attributes(&self) -> Vec<Attribute> {
    self.public_attributes.clone()
  }

  pub fn n_attributes(&self) -> u32 {
    self.n_attributes
  }

}
