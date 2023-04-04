use crate::{
    error::{BackendError, Result},
    state::State,
};
use nym_client_core::{client::key_manager::KeyManager, config::Config as BaseConfig};
use nym_config_common::NymConfig;
use nym_crypto::asymmetric::identity;
use nym_socks5_client_core::config::{Config as Socks5Config, Socks5};
use std::path::PathBuf;
use std::sync::Arc;
use tap::TapFallible;
use tokio::sync::RwLock;

static SOCKS5_CONFIG_ID: &str = "nym-connect";

pub fn socks5_config_id_appended_with(gateway_id: &str) -> Result<String> {
    use std::fmt::Write as _;
    let mut id = SOCKS5_CONFIG_ID.to_string();
    write!(id, "-{gateway_id}")?;
    Ok(id)
}

#[tauri::command]
pub async fn get_config_id(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<String> {
    state.read().await.get_config_id()
}

#[tauri::command]
pub fn get_config_file_location() -> Result<String> {
    Err(BackendError::CouldNotGetConfigFilename)
}

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

    pub async fn init(
        service_provider: &str,
        chosen_gateway_id: &str,
    ) -> Result<(Config, KeyManager)> {
        log::info!("Initialising...");

        let service_provider = service_provider.to_owned();
        let chosen_gateway_id = chosen_gateway_id.to_owned();

        // The client initialization was originally not written for this use case, so there are
        // lots of ways it can panic. Until we have proper error handling in the init code for the
        // clients we'll catch any panics here by spawning a new runtime in a separate thread.
        let (config, keys) = std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime")
                .block_on(
                    async move { init_socks5_config(service_provider, chosen_gateway_id).await },
                )
        })
        .join()
        .map_err(|_| BackendError::InitializationPanic)??;

        log::info!("Configuration saved 🚀");
        Ok((config, keys))
    }
}

pub async fn init_socks5_config(
    provider_address: String,
    chosen_gateway_id: String,
) -> Result<(Config, KeyManager)> {
    log::trace!("Initialising client...");
    let mut config = Config::new(SOCKS5_CONFIG_ID, &provider_address);

    if let Ok(raw_validators) = std::env::var(nym_config_common::defaults::var_names::NYM_API) {
        config
            .get_base_mut()
            .set_custom_nym_apis(nym_config_common::parse_urls(&raw_validators));
    }

    let nym_api_endpoints = config.get_base().get_nym_api_endpoints();

    let chosen_gateway_id = identity::PublicKey::from_base58_string(chosen_gateway_id)
        .map_err(|_| BackendError::UnableToParseGateway)?;

    let mut key_manager = nym_client_core::init::new_client_keys();

    // Setup gateway and register a new key each time
    let gateway = nym_client_core::init::register_with_gateway::<mobile_storage::EphemeralStorage>(
        &mut key_manager,
        nym_api_endpoints,
        Some(chosen_gateway_id),
        false,
    )
    .await?;

    config.get_base_mut().set_gateway_endpoint(gateway);

    print_saved_config(&config);

    let address = *key_manager.identity_keypair().public_key();
    log::info!("The address of this client is: {}", address);

    Ok((config, key_manager))
}

fn print_saved_config(config: &Config) {
    log::info!(
        "Saved configuration file to {:?}",
        config.get_config().get_config_file_save_location()
    );
    log::info!("Gateway id: {}", config.get_base().get_gateway_id());
    log::info!("Gateway owner: {}", config.get_base().get_gateway_owner());
    log::info!(
        "Gateway listener: {}",
        config.get_base().get_gateway_listener()
    );
    log::info!(
        "Service provider address: {}",
        config.get_socks5().get_provider_mix_address()
    );
    log::info!(
        "Service provider port: {}",
        config.get_socks5().get_listening_port()
    );
    log::info!("Client configuration completed.");
}
