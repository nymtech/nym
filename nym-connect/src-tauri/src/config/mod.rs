use client_core::config::GatewayEndpoint;
use log::info;
use once_cell::sync::Lazy;
use rand::Rng;

use client_core::config::Config as BaseConfig;
use config::NymConfig;
use nym_socks5::client::config::Config as Socks5Config;

// Generate a random id used for the config, since we need to init a new configuration each time
// due to not being able to reuse gateway registration. This is probably something we should
// improve.
pub static SOCKS5_CONFIG_ID: Lazy<String> = Lazy::new(|| {
    let mut rng = rand::thread_rng();
    format!("{}{:08}", "nym-connect-", rng.gen::<u64>())
});

// TODO: make this configurable from the UI
pub static PROVIDER_ADDRESS: &str = "EWa8DgePKfuWSjqPo6NEdavBK6gpnK4TKb2npi2HWuC2.6PGVT9y83UMGbFrPKDnCvTP2jJjpXYpD87ZpiRsLo1YR@CgQrYP8etksSBf4nALNqp93SHPpgFwEUyTsjBNNLj5WM";

const DEFAULT_ETH_ENDPOINT: &str = "https://rinkeby.infura.io/v3/00000000000000000000000000000000";
const DEFAULT_ETH_PRIVATE_KEY: &str =
    "0000000000000000000000000000000000000000000000000000000000000001";

pub struct Config {
    socks5: Socks5Config,
}

impl Config {
    pub fn new<S: Into<String>>(id: S, provider_mix_address: S) -> Self {
        Config {
            socks5: Socks5Config::new(id, provider_mix_address),
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

    pub async fn init() {
        info!("Initialising...");
        init_socks5(PROVIDER_ADDRESS, None).await;
        info!("Configuration saved 🚀");
    }
}

pub async fn init_socks5(provider_address: &str, chosen_gateway_id: Option<&str>) {
    let id: &str = &SOCKS5_CONFIG_ID;
    log::trace!("Creating config for id: {}", id);
    let mut config = Config::new(id, provider_address);

    let gateway = setup_gateway(chosen_gateway_id, config.get_socks5()).await;
    config.get_base_mut().with_gateway_endpoint(gateway);

    // As far as I'm aware, these two are not used, they are only set because the socks5 init code
    // requires them for initialising the bandwidth controller.
    config
        .get_base_mut()
        .with_eth_endpoint(DEFAULT_ETH_ENDPOINT);
    config
        .get_base_mut()
        .with_eth_private_key(DEFAULT_ETH_PRIVATE_KEY);

    let config_save_location = config.get_socks5().get_config_file_save_location();
    config
        .get_socks5()
        .save_to_file(None)
        .expect("Failed to save the config file");
    info!("Saved configuration file to {:?}", config_save_location);
    info!(
        "Using gateway: {}",
        config.get_socks5().get_base().get_gateway_id(),
    );
    info!("Client configuration completed.");

    client_core::init::show_address(config.get_base());
}

async fn setup_gateway(
    user_chosen_gateway_id: Option<&str>,
    config: &nym_socks5::client::config::Config,
) -> GatewayEndpoint {
    // Get the gateway details by querying the validator-api. Either pick one at random or use
    // the chosen one if it's among the available ones.
    log::info!("Configuring gateway");
    let gateway = client_core::init::query_gateway_details(
        config.get_base().get_validator_api_endpoints(),
        user_chosen_gateway_id,
    )
    .await;
    log::debug!("Querying gateway gives: {}", gateway);

    // Registering with gateway by setting up and writing shared keys to disk
    log::trace!("Registering gateway");
    client_core::init::register_with_gateway_and_store_keys(gateway.clone(), config.get_base())
        .await;
    log::info!("Saved all generated keys");

    gateway.into()
}
