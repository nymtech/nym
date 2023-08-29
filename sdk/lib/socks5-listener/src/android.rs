use crate::ClientState;
use ::safer_ffi::prelude::*;
use jni::{
    objects::{JClass, JObject, JString},
    sys::jint,
    JNIEnv,
};
use std::sync::{Arc, Mutex};

fn init_jni_logger() {
    use android_logger::{Config, FilterBuilder};
    use log::LevelFilter;

    android_logger::init_once(
        Config::default()
            .with_max_level(LevelFilter::Trace)
            .with_tag("libnyms5")
            .with_filter(
                FilterBuilder::new()
                    .parse("debug,tungstenite=warn,mio=warn,tokio_tungstenite=warn")
                    .build(),
            ),
    );
    log::debug!("Logger initialized");
}

/// Blocking call that starts the socks5 listener
#[no_mangle]
pub unsafe extern "C" fn Java_net_nymtech_nyms5_NymProxy_startClient(
    mut env: JNIEnv,
    _class: JClass,
    service_provider: JString,
    start_cb: JObject,
    stop_cb: JObject,
) {
    init_jni_logger();

    let sp_input: String = env
        .get_string(&service_provider)
        .expect("Couldn't get java string!")
        .into();

    log::debug!("using sp {}", sp_input);

    let service_provider = char_p::new(sp_input.as_str());

    let arced = Arc::new(Mutex::new(env));
    let env_start = arced.clone();
    let env_stop = arced.clone();

    crate::blocking_run_client(
        None,
        Some(service_provider.as_ref()),
        move |_| {
            log::debug!("client connected");
            env_start
                .lock()
                .unwrap()
                .call_method(&start_cb, "onStart", "()V", &[])
                .expect("failed to call Java callbacks");
        },
        move || {
            log::debug!("client disconnected");
            env_stop
                .lock()
                .unwrap()
                .call_method(&stop_cb, "onStop", "()V", &[])
                .expect("failed to call Java callbacks");
        },
    );
}

#[no_mangle]
pub unsafe extern "C" fn Java_net_nymtech_nyms5_NymProxy_stopClient(_env: JNIEnv, _class: JClass) {
    crate::stop_client();
}

#[no_mangle]
pub unsafe extern "C" fn Java_net_nymtech_nyms5_NymProxy_getClientState(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    let state = crate::get_client_state();
    log::debug!("client state {:?}", state);

    match state {
        ClientState::Uninitialised => 0,
        ClientState::Connected => 1,
        ClientState::Disconnected => 2,
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_net_nymtech_nyms5_NymProxy_pingClient(_env: JNIEnv, _class: JClass) {
    log::debug!("pong");
    crate::ping_client();
}
