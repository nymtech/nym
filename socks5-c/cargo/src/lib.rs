use ::safer_ffi::prelude::*;
use anyhow::Result;
use lazy_static::lazy_static;
use nym_client_core::client::key_manager::KeyManager;
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

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
pub mod android {
    extern crate jni;

    use std::ffi::CString;

    use self::jni::objects::{JClass, JString};
    use self::jni::sys::jstring;
    use self::jni::JNIEnv;
    use super::*;

    extern "C" fn placeholder_cb() {}

    #[no_mangle]
    pub unsafe extern "C" fn Java_net_nymtech_nyms5_Socks5_runclient(
        env: JNIEnv,
        _: JClass,
        java_pattern: JString,
    ) {
        let fake_service_provider = "foomp".to_string();
        blocking_run_client(fake_service_provider, placeholder_cb, placeholder_cb)
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
    service_provider: char_p::Ref<'_>,
    on_start_callback: extern "C" fn(),
    on_shutdown_callback: extern "C" fn(),
) {
    let service_provider = service_provider.to_string();
    RUNTIME.spawn(async move {
        _async_run_client(service_provider, on_start_callback, on_shutdown_callback).await
    });
}

#[ffi_export]
pub fn stop_client() {
    RUNTIME.block_on(async move { stop_and_reset_shutdown_handle().await })
}

#[ffi_export]
pub fn blocking_run_client(
    service_provider: char_p::Ref<'_>,
    on_start_callback: extern "C" fn(),
    on_shutdown_callback: extern "C" fn(),
) {
    let service_provider = service_provider.to_string();
    RUNTIME
        .block_on(async move {
            _async_run_client(service_provider, on_start_callback, on_shutdown_callback).await
        })
        .unwrap();
}

#[ffi_export]
pub fn write_to_file(dir: char_p::Ref<'_>, id: char_p::Ref<'_>, service_provider: char_p::Ref<'_>) {
    let cfg = Config::new(id.to_string(), service_provider.to_string());
    cfg.save_to_file(Some(PathBuf::from(dir.to_string())))
        .expect("failed to save config")
}

#[ffi_export]
pub fn read_from_file(dir: char_p::Ref<'_>) -> char_p::Box {
    let cfg =
        Config::load_from_filepath(PathBuf::from(dir.to_string())).expect("failed to load config");
    format!("{:#?}", cfg).try_into().unwrap()
    //
}

async fn _async_run_client(
    service_provider: String,
    on_start_callback: extern "C" fn(),
    on_shutdown_callback: extern "C" fn(),
) -> anyhow::Result<()> {
    set_default_env();
    let stop_handle = Arc::new(Notify::new());
    set_shutdown_handle(stop_handle.clone()).await;

    let (config, keys) = init_dummy_socks5_config(service_provider).await.unwrap();
    let socks5_client = Socks5NymClient::new_with_keys(config, Some(keys));
    let mut shutdown_handle = socks5_client.start().await?;

    // invoke the callback since we've started!
    on_start_callback();

    // wait for notify to be set...
    stop_handle.notified().await;

    // and then do graceful shutdown of all tasks
    shutdown_handle.signal_shutdown().ok();
    shutdown_handle.wait_for_shutdown().await;

    // and the corresponding one for shutdown!
    on_shutdown_callback();

    // shutdown_handle
    //     .catch_interrupt()
    //     .await
    //     .expect("failed to catch interrupt");

    Ok(())
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
