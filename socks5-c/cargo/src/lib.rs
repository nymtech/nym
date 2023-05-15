use ::safer_ffi::prelude::*;
use anyhow::Result;
use nym_client_core::client::key_manager::KeyManager;
use nym_config_common::defaults::setup_env;
use nym_credential_storage::ephemeral_storage::EphemeralStorage;
use nym_socks5_client_core::config::Config as Socks5Config;
use nym_socks5_client_core::NymClient as Socks5NymClient;

static SOCKS5_CONFIG_ID: &str = "mobile-socks5-test";

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
pub mod android {
    extern crate jni;

    use std::ffi::CString;

    use self::jni::objects::{JClass, JString};
    use self::jni::sys::jstring;
    use self::jni::JNIEnv;
    use super::*;

    #[no_mangle]
    pub unsafe extern "C" fn Java_net_nymtech_nyms5_Socks5_runclient(
        env: JNIEnv,
        _: JClass,
        java_pattern: JString,
    ) {
        let fake_service_provider = "foomp".to_string();
        _run_client(fake_service_provider)
    }
}

// #[cfg(target_os = "ios")]
// #[swift_bridge::bridge]
// pub mod ios {
//     extern "Rust" {
//         type Socks5Client;
//
//         #[swift_bridge(init)]
//         fn new() -> Socks5Client;
//
//         fn foomp(&self, val: &str) -> String;
//     }
// }
//
// pub struct Socks5Client;
//
// impl Socks5Client {
//     fn new() -> Self {
//         Socks5Client
//     }
//
//     fn foomp(&self, val: &str) -> String {
//         format!("{val} with extra foomp")
//     }
// }

#[derive_ReprC]
#[ffi_export]
#[repr(C)]
pub enum ClientState {
    Unknown,
    Connected,
    Disconnected,
}

#[ffi_export]
pub fn foomp(val: char_p::Ref<'_>) -> char_p::Box {
    format!("{val} with extra foomp").try_into().unwrap()
}

#[ffi_export]
pub fn free_foomp(foomp: char_p::Box) {
    drop(foomp)
}

#[ffi_export]
pub fn run_client(service_provider: char_p::Ref<'_>) {
    let service_provider = service_provider.to_string();
    _run_client(service_provider)
}

fn _run_client(service_provider: String) {
    setup_env(None);
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async move {
        let (config, keys) = init_dummy_socks5_config(service_provider).await.unwrap();
        let socks5_client = Socks5NymClient::new_with_keys(config, Some(keys));
        let shutdown_handle = socks5_client.start().await?;
        shutdown_handle
            .catch_interrupt()
            .await
            .expect("failed to catch interrupt");

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

#[cfg(feature = "headers")] // c.f. the `Cargo.toml` section
pub fn generate_headers() -> ::std::io::Result<()> {
    ::safer_ffi::headers::builder()
        .to_file("socks5_c.h")?
        .generate()
}
