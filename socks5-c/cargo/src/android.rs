use ::safer_ffi::prelude::*;
use jni::{
    objects::{JClass, JString},
    sys::jstring,
    JNIEnv,
};

extern "C" fn placeholder_startup_cb(address: char_p::Box) {
    crate::rust_free_string(address)
}

extern "C" fn placeholder_shutdown_cb() {}

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
pub unsafe extern "C" fn Java_net_nymtech_nyms5_Socks5_run(
    mut env: JNIEnv,
    _class: JClass,
    input: JString,
) -> jstring {
    init_jni_logger();

    // We don't really make use of this input right now, but still keep it as we are expected
    // to make use of it in the future.
    let input: String = env
        .get_string(&input)
        .expect("Couldn't get java string!")
        .into();

    // TODO: get the service provider from input
    let service_provider = char_p::new("DpB3cHAchJiNBQi5FrZx2csXb1mrHkpYh9Wzf8Rjsuko.ANNWrvHqMYuertHGHUrZdBntQhpzfbWekB39qez9U2Vx@2BuMSfMW3zpeAjKXyKLhmY4QW1DXurrtSPEJ6CjX3SEh");
    crate::blocking_run_client(
        None,
        Some(service_provider.as_ref()),
        placeholder_startup_cb,
        placeholder_shutdown_cb,
    );

    // Return something here not because we need to, but because we will likely do so in the
    // future.
    let output = env
        .new_string(format!("Hello, {}!", input))
        .expect("Couldn't create java string!");

    output.into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn Java_net_nymtech_nyms5_Socks5_start_client(
    _env: JNIEnv,
    _class: JClass,
    _input: JString,
) {
    // TODO: how does this work if we are not doing blocking calls?
    init_jni_logger();

    // TODO: get the service provider from input
    let service_provider = char_p::new("DpB3cHAchJiNBQi5FrZx2csXb1mrHkpYh9Wzf8Rjsuko.ANNWrvHqMYuertHGHUrZdBntQhpzfbWekB39qez9U2Vx@2BuMSfMW3zpeAjKXyKLhmY4QW1DXurrtSPEJ6CjX3SEh");
    crate::start_client(
        None,
        Some(service_provider.as_ref()),
        placeholder_startup_cb,
        placeholder_shutdown_cb,
    );
}

#[no_mangle]
pub unsafe extern "C" fn Java_net_nymtech_nyms5_Socks5_stop_client(
    _env: JNIEnv,
    _class: JClass,
) {
    //init_jni_logger();
    crate::stop_client();
}
