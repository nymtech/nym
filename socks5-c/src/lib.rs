use anyhow::Result;
use nym_client_core::{client::key_manager::KeyManager, config::Config as BaseConfig};
use nym_credential_storage::ephemeral_storage::EphemeralStorage;
use nym_crypto::asymmetric::identity;
use nym_socks5_client_core::config::Config as Socks5Config;
use nym_socks5_client_core::NymClient as Socks5NymClient;
//use std::ffi::*;

static SOCKS5_CONFIG_ID: &str = "nym-connect";

// /// # Safety
// ///
// /// TODO
// #[no_mangle]
// pub unsafe extern "C" fn run_client(service_provider: *const c_char) {
//     // TODO: does that leak memory and do we have to have a separate free method?
//     let c_str = unsafe { CStr::from_ptr(service_provider) };
//     let service_provider = c_str
//         .to_str()
//         .expect("invalid service provider string value provided");
//
//     let gateway = "".to_string();
//
//     let rt = tokio::runtime::Runtime::new().unwrap();
//
//     rt.block_on(async move {
//         let (config, keys) = init_socks5_config(service_provider, gateway).await.unwrap();
//         let socks5_client = Socks5NymClient::new_with_keys(config.socks5, Some(keys));
//         socks5_client.run_and_listen2().await
//     })
//     .unwrap();
// }

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn rust_greeting(to: *const c_char) -> *mut c_char {
    // let service_provider = "Entztfv6Uaz2hpYHQJ6JKoaCTpDL5dja18SuQWVJAmmx.Cvhn9rBJw5Ay9wgHcbgCnVg89MPSV5s2muPV2YF1BXYu@Fo4f4SQLdoyoGkFae5TpVhRVoXCF8UiypLVGtGjujVPf".to_string();
    let service_provider = "4z4iw9NLRgMok2MPFEGoiwrmHuDY6kRVDUQRp2dXGLQm.69av5mWZmaMK4bHo3GV6Cu7B8zuMT2mv2E22f8GkRMgk@DF4TE7V8kJkttMvnoSVGnRFFRt6WYGxxiC2w1XyPQnHe".to_string();
    let gateway = "Fo4f4SQLdoyoGkFae5TpVhRVoXCF8UiypLVGtGjujVPf".to_string();

    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async move {
        let (config, keys) = init_socks5_config(service_provider, gateway).await.unwrap();
        let socks5_client = Socks5NymClient::new_with_keys(config.socks5, Some(keys));
        socks5_client.run_and_listen2().await
    })
    .unwrap();

    CString::new("Hello ").unwrap().into_raw()
}

/*
#[no_mangle]
pub extern fn rust_greeting(to: *const c_char) -> *mut c_char {
    let c_str = unsafe { CStr::from_ptr(to) };
    let recipient = match c_str.to_str() {
        Err(_) => "there",
        Ok(string) => string,
    };

    CString::new("Hello ".to_owned() + recipient).unwrap().into_raw()
}
 */

#[no_mangle]
pub extern "C" fn rust_greeting_free(s: *mut c_char) {
    unsafe {
        if s.is_null() {
            return;
        }
        CString::from_raw(s)
    };
}

// const char* rust_greeting(const char* to);
// void rust_greeting_free(char *);

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

    let _chosen_gateway_id = identity::PublicKey::from_base58_string(chosen_gateway_id)?;

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

    let _address = *key_manager.identity_keypair().public_key();

    Ok((config, key_manager))
}
