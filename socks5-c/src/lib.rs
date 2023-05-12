use anyhow::Result;
use nym_client_core::client::key_manager::KeyManager;
use nym_config_common::defaults::setup_env;
use nym_credential_storage::ephemeral_storage::EphemeralStorage;
use nym_socks5_client_core::config::Config as Socks5Config;
use nym_socks5_client_core::NymClient as Socks5NymClient;
use std::ffi::CStr;
use std::os::raw::c_char;

static SOCKS5_CONFIG_ID: &str = "mobile-socks5-test";

/// # Safety
///
/// TODO
#[no_mangle]
pub unsafe extern "C" fn run_client(service_provider: *const c_char) {
    setup_env(None);

    // TODO: does that leak memory and do we have to have a separate free method?
    // I'd assume not because the allocation came from the caller
    let c_str = unsafe { CStr::from_ptr(service_provider) };
    let service_provider = c_str
        .to_str()
        .expect("invalid service provider string value provided")
        .to_string();

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async move {
        let (config, keys) = init_dummy_socks5_config(service_provider).await.unwrap();
        let socks5_client = Socks5NymClient::new_with_keys(config, Some(keys));
        let mut shutdown_handle = socks5_client.start().await?;
        shutdown_handle.wait_for_shutdown().await;

        Ok::<(), anyhow::Error>(())
    })
    .unwrap();
}

pub async fn init_dummy_socks5_config(
    provider_address: String,
    // chosen_gateway_id: String,
) -> Result<(Socks5Config, KeyManager)> {
    let mut config = Socks5Config::new(SOCKS5_CONFIG_ID, &provider_address);

    if let Ok(raw_validators) = std::env::var(nym_config_common::defaults::var_names::NYM_API) {
        config
            .get_base_mut()
            .set_custom_nym_apis(nym_config_common::parse_urls(&raw_validators));
    }

    let nym_api_endpoints = config.get_base().get_nym_api_endpoints();

    // let _chosen_gateway_id = identity::PublicKey::from_base58_string(chosen_gateway_id)?;

    let mut key_manager = nym_client_core::init::new_client_keys();

    // Setup gateway and register a new key each time
    let gateway = nym_client_core::init::register_with_gateway::<EphemeralStorage>(
        &mut key_manager,
        nym_api_endpoints,
        //Some(chosen_gateway_id),
        None,
        false,
    )
    .await?;

    config.get_base_mut().set_gateway_endpoint(gateway);

    // let _address = *key_manager.identity_keypair().public_key();

    Ok((config, key_manager))
}
