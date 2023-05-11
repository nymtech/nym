use nym_socks5_client_core::NymClient as Socks5NymClient;
use std::ffi::*;

#[no_mangle]
pub extern "C" fn run_client() {
    let service_provider = "".to_string();
    let gateway = "".to_string();

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async move {
        let (config, keys) = init_socks5_config(service_provider, gateway).await.unwrap();
        let socks5_client = Socks5NymClient::new_with_keys(config.socks5, Some(keys));
        socks5_client.run_and_listen2().await
    })
    .unwrap();
}

use anyhow::Result;
use futures::channel::mpsc;
use nym_client_core::config::{ClientCoreConfigTrait, GatewayEndpointConfig};
use nym_client_core::{client::key_manager::KeyManager, config::Config as BaseConfig};
use nym_config_common::NymConfig;
use nym_credential_storage::ephemeral_storage::EphemeralStorage;
use nym_crypto::asymmetric::identity;
use nym_socks5_client_core::config::{Config as Socks5Config, Socks5};
use nym_socks5_client_core::Socks5ControlMessageSender;
use std::path::PathBuf;
use std::sync::Arc;
use tap::TapFallible;
use tokio::sync::RwLock;

static SOCKS5_CONFIG_ID: &str = "nym-connect";

#[derive(Debug)]
pub struct Config {
    pub socks5: Socks5Config,
}

impl Config {
    pub fn new<S: Into<String>>(id: S, provider_mix_address: S) -> Self {
        Config {
            socks5: Socks5Config::new(id, provider_mix_address),
        }
    }

    #[allow(unused)]
    pub fn new_with_port<S: Into<String>>(id: S, provider_mix_address: S, port: u16) -> Self {
        Config {
            socks5: Socks5Config::new(id, provider_mix_address).with_port(port),
        }
    }

    pub fn get_config(&self) -> &Socks5Config {
        &self.socks5
    }

    pub fn get_socks5(&self) -> &Socks5 {
        self.socks5.get_socks5()
    }

    #[allow(unused)]
    pub fn get_socks5_mut(&mut self) -> &mut Socks5 {
        self.socks5.get_socks5_mut()
    }

    pub fn get_base(&self) -> &BaseConfig<Socks5Config> {
        self.socks5.get_base()
    }

    pub fn get_base_mut(&mut self) -> &mut BaseConfig<Socks5Config> {
        self.socks5.get_base_mut()
    }
}

pub async fn init_socks5_config(
    provider_address: String,
    chosen_gateway_id: String,
) -> Result<(Config, KeyManager)> {
    let mut config = Config::new(SOCKS5_CONFIG_ID, &provider_address);

    if let Ok(raw_validators) = std::env::var(nym_config_common::defaults::var_names::NYM_API) {
        config
            .get_base_mut()
            .set_custom_nym_apis(nym_config_common::parse_urls(&raw_validators));
    }

    let nym_api_endpoints = config.get_base().get_nym_api_endpoints();

    let chosen_gateway_id = identity::PublicKey::from_base58_string(chosen_gateway_id)?;

    let mut key_manager = nym_client_core::init::new_client_keys();

    // Setup gateway and register a new key each time
    let gateway = nym_client_core::init::register_with_gateway::<EphemeralStorage>(
        &mut key_manager,
        nym_api_endpoints,
        Some(chosen_gateway_id),
        false,
    )
    .await?;

    config.get_base_mut().set_gateway_endpoint(gateway);

    let address = *key_manager.identity_keypair().public_key();

    Ok((config, key_manager))
}
