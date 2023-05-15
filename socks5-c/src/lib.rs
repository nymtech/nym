use anyhow::Result;
use nym_client_core::client::key_manager::KeyManager;
use nym_config_common::defaults::setup_env;
use nym_credential_storage::ephemeral_storage::EphemeralStorage;
use nym_socks5_client_core::config::Config as Socks5Config;
use nym_socks5_client_core::NymClient as Socks5NymClient;
use std::ffi::CStr;
use std::os::raw::c_char;

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

    use android_logger::Config;
    use log::Level;

    #[no_mangle]
    #[allow(non_snake_case)]
    pub extern "C" fn Java_net_nymtech_nyms5_Socks5_run<'local>(
        mut env: JNIEnv<'local>,
        class: JClass<'local>,
        input: JString<'local>,
    ) -> jstring {
        android_logger::init_once(
            Config::default().with_min_level(Level::Trace), // minimum log level
        );
        log::debug!("Logger initialized");

        let input: String = env
            .get_string(&input)
            .expect("Couldn't get java string!")
            .into();

        setup_env(None);

        // let mut log_builder = pretty_env_logger::formatted_timed_builder();
        // if let Ok(s) = ::std::env::var("RUST_LOG") {
        //     log_builder.parse_filters(&s);
        // } else {
        //     // default to 'Info'
        //     log_builder.filter(None, log::LevelFilter::Info);
        // }

        // log_builder
        //     .filter_module("hyper", log::LevelFilter::Warn)
        //     .filter_module("tokio_reactor", log::LevelFilter::Warn)
        //     .filter_module("reqwest", log::LevelFilter::Warn)
        //     .filter_module("mio", log::LevelFilter::Warn)
        //     .filter_module("want", log::LevelFilter::Warn)
        //     .filter_module("tungstenite", log::LevelFilter::Warn)
        //     .filter_module("tokio_tungstenite", log::LevelFilter::Warn)
        //     .filter_module("handlebars", log::LevelFilter::Warn)
        //     .filter_module("sled", log::LevelFilter::Warn)
        //     .init();

        // log::info!("HEREE****");

        // TODO: does that leak memory and do we have to have a separate free method?
        // I'd assume not because the allocation came from the caller
        // let c_str = unsafe { CStr::from_ptr(service_provider) };
        let c_str = CString::new("DpB3cHAchJiNBQi5FrZx2csXb1mrHkpYh9Wzf8Rjsuko.ANNWrvHqMYuertHGHUrZdBntQhpzfbWekB39qez9U2Vx@2BuMSfMW3zpeAjKXyKLhmY4QW1DXurrtSPEJ6CjX3SEh").unwrap();
        let service_provider = c_str
            .to_str()
            .expect("invalid service provider string value provided")
            .to_string();

        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async move {
            let (config, keys) = init_dummy_socks5_config(service_provider).await.unwrap();
            // let socks5_client = Socks5NymClient::new_with_keys(config, Some(keys));
            // let mut shutdown_handle = socks5_client.start().await?;
            // shutdown_handle.wait_for_shutdown().await;

            Ok::<(), anyhow::Error>(())
        })
        .unwrap();

        let output = env
            .new_string(format!("Hello, {}!", input))
            .expect("Couldn't create java string!");

        output.into_raw()
    }

    #[no_mangle]
    pub unsafe extern "C" fn Java_net_nymtech_nyms5_Socks5_runclient(
        env: JNIEnv,
        _: JClass,
        java_pattern: JString,
    ) {
        // setup_env(None);

        // TODO: does that leak memory and do we have to have a separate free method?
        // I'd assume not because the allocation came from the caller
        //let c_str = unsafe { CStr::from_ptr(service_provider) };
        // let c_str = CString::new("DpB3cHAchJiNBQi5FrZx2csXb1mrHkpYh9Wzf8Rjsuko.ANNWrvHqMYuertHGHUrZdBntQhpzfbWekB39qez9U2Vx@2BuMSfMW3zpeAjKXyKLhmY4QW1DXurrtSPEJ6CjX3SEh").unwrap();
        // let service_provider = c_str
        //     .to_str()
        //     .expect("invalid service provider string value provided")
        //     .to_string();

        // let rt = tokio::runtime::Runtime::new().unwrap();

        // rt.block_on(async move {
        //     let (config, keys) = init_dummy_socks5_config(service_provider).await.unwrap();
        //     let socks5_client = Socks5NymClient::new_with_keys(config, Some(keys));
        //     let mut shutdown_handle = socks5_client.start().await?;
        //     shutdown_handle.wait_for_shutdown().await;

        //     Ok::<(), anyhow::Error>(())
        // })
        // .unwrap();
    }
}

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
    // let gateway = nym_client_core::init::register_with_gateway::<EphemeralStorage>(
    //     &mut key_manager,
    //     nym_api_endpoints,
    //     //Some(chosen_gateway_id),
    //     None,
    //     false,
    // )
    // .await?;

    // config.get_base_mut().set_gateway_endpoint(gateway);

    // let _address = *key_manager.identity_keypair().public_key();

    Ok((config, key_manager))
}
