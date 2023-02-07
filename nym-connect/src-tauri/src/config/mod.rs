use crate::{
    error::{BackendError, Result},
    state::State,
};
use client_core::config::Config as BaseConfig;
use config_common::NymConfig;
use crypto::asymmetric::identity;
use nym_socks5::client::config::Config as Socks5Config;
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
pub async fn get_config_file_location(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<String> {
    let id = get_config_id(state).await?;
    Config::config_file_location(&id).map(|d| d.to_string_lossy().to_string())
}

#[derive(Debug)]
pub struct Config {
    socks5: Socks5Config,
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

    pub fn get_socks5(&self) -> &Socks5Config {
        &self.socks5
    }

    #[allow(unused)]
    pub fn get_socks5_mut(&mut self) -> &mut Socks5Config {
        &mut self.socks5
    }

    pub fn get_base(&self) -> &BaseConfig<Socks5Config> {
        self.socks5.get_base()
    }

    pub fn get_base_mut(&mut self) -> &mut BaseConfig<Socks5Config> {
        self.socks5.get_base_mut()
    }

    pub async fn init(service_provider: &str, chosen_gateway_id: &str) -> Result<()> {
        log::info!("Initialising...");

        let service_provider = service_provider.to_owned();
        let chosen_gateway_id = chosen_gateway_id.to_owned();

        // The client initialization was originally not written for this use case, so there are
        // lots of ways it can panic. Until we have proper error handling in the init code for the
        // clients we'll catch any panics here by spawning a new runtime in a separate thread.
        std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime")
                .block_on(
                    async move { init_socks5_config(service_provider, chosen_gateway_id).await },
                )
        })
        .join()
        .map_err(|_| BackendError::InitializationPanic)??;

        log::info!("Configuration saved 🚀");
        Ok(())
    }

    pub fn config_file_location(id: &str) -> Result<PathBuf> {
        Socks5Config::try_default_config_file_path(id)
            .ok_or(BackendError::CouldNotGetConfigFilename)
    }
}

pub async fn init_socks5_config(provider_address: String, chosen_gateway_id: String) -> Result<()> {
    log::trace!("Initialising client...");

    // Append the gateway id to the name id that we store the config under
    let id = socks5_config_id_appended_with(&chosen_gateway_id)?;

    log::debug!(
        "Attempting to use config file location: {}",
        Config::config_file_location(&id)?.to_string_lossy(),
    );
    let already_init = Config::config_file_location(&id)?.exists();
    if already_init {
        log::info!("SOCKS5 client \"{id}\" was already initialised before");
    }

    // Future proofing. This flag exists for the other clients
    let user_wants_force_register = false;

    // If the client was already initialized, don't generate new keys and don't re-register with
    // the gateway (because this would create a new shared key).
    // Unless the user really wants to.
    let register_gateway = !already_init || user_wants_force_register;

    log::trace!("Creating config for id: {}", id);
    let mut config = Config::new(id.as_str(), &provider_address);

    if let Ok(raw_validators) = std::env::var(config_common::defaults::var_names::NYM_API) {
        config
            .get_base_mut()
            .set_custom_nym_apis(config_common::parse_urls(&raw_validators));
    }

    let chosen_gateway_id = identity::PublicKey::from_base58_string(chosen_gateway_id)
        .map_err(|_| BackendError::UnableToParseGateway)?;

    // Setup gateway by either registering a new one, or reusing exiting keys
    let gateway = client_core::init::setup_gateway_from_config::<Socks5Config, _>(
        register_gateway,
        Some(chosen_gateway_id),
        config.get_base(),
    )
    .await?;

    config.get_base_mut().set_gateway_endpoint(gateway);

    config.get_socks5().save_to_file(None).tap_err(|_| {
        log::error!("Failed to save the config file");
    })?;

    print_saved_config(&config);

    let address = client_core::init::get_client_address_from_stored_keys(config.get_base())?;
    log::info!("The address of this client is: {}", address);
    Ok(())
}

fn print_saved_config(config: &Config) {
    log::info!(
        "Saved configuration file to {:?}",
        config.get_socks5().get_config_file_save_location()
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
