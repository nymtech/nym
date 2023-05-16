// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use ::safer_ffi::prelude::*;
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use nym_config_common::defaults::setup_env;
use nym_config_common::NymConfig;
use nym_credential_storage::ephemeral_storage::EphemeralStorage;
use nym_socks5_client_core::config::{Config as Socks5Config, Config};
use nym_socks5_client_core::NymClient as Socks5NymClient;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::{Mutex, Notify};

static SOCKS5_CONFIG_ID: &str = "mobile-socks5-test";

// return address of the client
type StartupCallback = extern "C" fn(char_p::Box);

type ShutdownCallback = extern "C" fn();

// hehe, this is so disgusting : )
lazy_static! {
    static ref CLIENT_SHUTDOWN_HANDLE: Mutex<Option<Arc<Notify>>> = Mutex::new(None);
    static ref RUNTIME: Runtime = Runtime::new().unwrap();
}
static ENV_SET: AtomicBool = AtomicBool::new(false);

async fn set_shutdown_handle(handle: Arc<Notify>) {
    let mut guard = CLIENT_SHUTDOWN_HANDLE.lock().await;
    if guard.is_some() {
        panic!("client wasn't properly stopped")
    }
    *guard = Some(handle)
}

async fn stop_and_reset_shutdown_handle() {
    let mut guard = CLIENT_SHUTDOWN_HANDLE.lock().await;
    if let Some(sh) = &*guard {
        sh.notify_waiters()
    } else {
        panic!("client wasn't properly started")
    }

    *guard = None
}

fn set_default_env() {
    if !ENV_SET.swap(true, Ordering::SeqCst) {
        setup_env(None);
    }
}

// to be used with the on startup callback which returns the address
#[ffi_export]
fn rust_free_string(string: char_p::Box) {
    drop(string)
}

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
pub mod android {
    extern crate jni;

    use std::ffi::CString;

    use self::jni::objects::{JClass, JString};
    use self::jni::sys::jstring;
    use self::jni::JNIEnv;
    use super::*;

    extern "C" fn placeholder_startup_cb(address: char_p::Box) {
        rust_free_string(address)
    }
    extern "C" fn placeholder_shutdown_cb() {}

    #[no_mangle]
    pub unsafe extern "C" fn Java_net_nymtech_nyms5_Socks5_runclient(
        env: JNIEnv,
        _: JClass,
        java_pattern: JString,
    ) {
        let fake_service_provider = "foomp".to_string();
        blocking_run_client(
            fake_service_provider,
            placeholder_startup_cb,
            placeholder_shutdown_cb,
        )
    }
}

#[derive_ReprC]
#[ffi_export]
#[repr(C)]
pub enum ClientState {
    Unknown,
    Connected,
    Disconnected,
}

#[ffi_export]
pub fn start_client(
    storage_directory: char_p::Ref<'_>,
    service_provider: Option<char_p::Ref<'_>>,
    on_start_callback: StartupCallback,
    on_shutdown_callback: ShutdownCallback,
) {
    let storage_dir = storage_directory.to_string();
    let service_provider = service_provider.map(|s| s.to_string());
    RUNTIME.spawn(async move {
        _async_run_client(
            storage_dir,
            SOCKS5_CONFIG_ID.to_string(),
            service_provider,
            on_start_callback,
            on_shutdown_callback,
        )
        .await
    });
}

#[ffi_export]
pub fn stop_client() {
    RUNTIME.block_on(async move { stop_and_reset_shutdown_handle().await })
}

#[ffi_export]
pub fn blocking_run_client(
    storage_directory: char_p::Ref<'_>,
    service_provider: Option<char_p::Ref<'_>>,
    on_start_callback: StartupCallback,
    on_shutdown_callback: ShutdownCallback,
) {
    let storage_dir = storage_directory.to_string();
    let service_provider = service_provider.map(|s| s.to_string());
    RUNTIME
        .block_on(async move {
            _async_run_client(
                storage_dir,
                SOCKS5_CONFIG_ID.to_string(),
                service_provider,
                on_start_callback,
                on_shutdown_callback,
            )
            .await
        })
        .unwrap();
}

#[ffi_export]
pub fn reset_client_data(root_directory: char_p::Ref<'_>) {
    let root_dir = root_directory.to_string();
    _reset_client_data(root_dir)
}

fn _reset_client_data(root_directory: String) {
    let client_storage_dir = PathBuf::new().join(root_directory).join(SOCKS5_CONFIG_ID);
    std::fs::remove_dir_all(client_storage_dir).expect("failed to clear client data")
}

// #[ffi_export]
// pub fn write_to_file(dir: char_p::Ref<'_>, id: char_p::Ref<'_>, service_provider: char_p::Ref<'_>) {
//     let cfg = Config::new(id.to_string(), service_provider.to_string())
//         .with_root_directory(dir.to_string());
//     cfg.save_to_file(None).expect("failed to save config")
// }
//
// #[ffi_export]
// pub fn read_from_file(dir: char_p::Ref<'_>) -> char_p::Box {
//     let cfg =
//         Config::load_from_filepath(PathBuf::from(dir.to_string())).expect("failed to load config");
//     format!("{:#?}", cfg).try_into().unwrap()
//     //
// }

async fn _async_run_client(
    storage_dir: String,
    client_id: String,
    service_provider: Option<String>,
    on_start_callback: StartupCallback,
    on_shutdown_callback: ShutdownCallback,
) -> anyhow::Result<()> {
    set_default_env();
    let stop_handle = Arc::new(Notify::new());
    set_shutdown_handle(stop_handle.clone()).await;

    // let (config, keys) = init_dummy_socks5_config(service_provider).await.unwrap();
    // let socks5_client = Socks5NymClient::new_with_keys(config, Some(keys));

    let config = load_or_generate_base_config(storage_dir, client_id, service_provider).await?;
    let socks5_client = Socks5NymClient::new(config);

    eprintln!("starting the socks5 client");
    let (self_address, mut shutdown_handle) = socks5_client.start().await?;
    eprintln!("the client has started!");

    // invoke the callback since we've started!
    on_start_callback(
        self_address
            .to_string()
            .try_into()
            .expect("malformed C string"),
    );

    // wait for notify to be set...
    stop_handle.notified().await;

    // and then do graceful shutdown of all tasks
    shutdown_handle.signal_shutdown().ok();
    shutdown_handle.wait_for_shutdown().await;

    // and the corresponding one for shutdown!
    on_shutdown_callback();

    Ok(())
}

// note: it does might not contain any gateway configuration and should not be persisted in that state!
async fn load_or_generate_base_config(
    storage_dir: String,
    client_id: String,
    service_provider: Option<String>,
) -> Result<Socks5Config> {
    let expected_store_path =
        Socks5Config::default_config_file_path_with_root(&storage_dir, &client_id.to_string());
    eprintln!("attempting to load socks5 config from {expected_store_path:?}");

    // simulator workaround
    if let Ok(config) = Socks5Config::load_from_filepath(expected_store_path) {
        eprintln!("loaded config");
        let root = config.get_base().get_nym_root_directory();
        eprintln!("actual root: {storage_dir}");
        eprintln!("retrieved root: {root:?}");

        if root.to_str() == Some(storage_dir.as_str()) {
            return Ok(config);
        }
        eprintln!("... but it seems to have been made for different container - fixing it up... (ASSUMING DEFAULT PATHS)");
        return Ok(config.with_root_directory(storage_dir));
    };

    eprintln!("creating new config");
    setup_new_client_config(storage_dir, client_id, service_provider).await
}

async fn setup_new_client_config(
    storage_dir: String,
    client_id: String,
    service_provider: Option<String>,
) -> Result<Socks5Config> {
    let service_provider = service_provider.ok_or(anyhow!(
        "service provider was not specified for fresh config"
    ))?;

    let mut new_config =
        Socks5Config::new(client_id, service_provider).with_root_directory(storage_dir);
    if let Ok(raw_validators) = std::env::var(nym_config_common::defaults::var_names::NYM_API) {
        new_config
            .get_base_mut()
            .set_custom_nym_apis(nym_config_common::parse_urls(&raw_validators));
    }

    // note: this will also do key storage (annoyingly...)
    let gateway = nym_client_core::init::setup_gateway_from_config::<Config, _, EphemeralStorage>(
        true,
        None,
        new_config.get_base(),
        false,
    )
    .await?;

    new_config.get_base_mut().set_gateway_endpoint(gateway);
    new_config.save_to_file(None)?;

    Ok(new_config)
}

//
// pub async fn init_dummy_socks5_config(
//     provider_address: String,
//     // base_storage_path: String,
//     // chosen_gateway_id: String,
// ) -> Result<(Socks5Config, KeyManager)> {
//     // let config_storage_path = Path::new(&base_storage_path).join("config");
//     // let data_storage_path = Path::new(&base_storage_path).join("data");
//
//     let mut config = Socks5Config::new(SOCKS5_CONFIG_ID, &provider_address);
//
//     if let Ok(raw_validators) = std::env::var(nym_config_common::defaults::var_names::NYM_API) {
//         config
//             .get_base_mut()
//             .set_custom_nym_apis(nym_config_common::parse_urls(&raw_validators));
//     }
//
//     let nym_api_endpoints = config.get_base().get_nym_api_endpoints();
//
//     // let _chosen_gateway_id = identity::PublicKey::from_base58_string(chosen_gateway_id)?;
//
//     let mut key_manager = nym_client_core::init::new_client_keys();
//
//     // Setup gateway and register a new key each time
//     let gateway = nym_client_core::init::register_with_gateway::<EphemeralStorage>(
//         &mut key_manager,
//         nym_api_endpoints,
//         //Some(chosen_gateway_id),
//         None,
//         false,
//     )
//     .await?;
//
//     config.get_base_mut().set_gateway_endpoint(gateway);
//
//     // let _address = *key_manager.identity_keypair().public_key();
//
//     Ok((config, key_manager))
// }

#[cfg(feature = "headers")] // c.f. the `Cargo.toml` section
pub fn generate_headers() -> ::std::io::Result<()> {
    ::safer_ffi::headers::builder()
        .to_file("socks5_c.h")?
        .generate()
}
