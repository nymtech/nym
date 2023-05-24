use ::safer_ffi::prelude::*;
use jni::{
    objects::{JClass, JObject, JString},
    JNIEnv,
};
use safer_ffi::char_p::char_p_boxed;
use safer_ffi::closure::{RefDynFnMut0, RefDynFnMut1};
use std::{thread, time};

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
pub unsafe extern "C" fn Java_net_nymtech_nyms5_NymProxy_run(
    mut env: JNIEnv,
    _class: JClass,
    service_provider: JString,
    cb_object: JObject,
) {
    init_jni_logger();

    let sp_input: String = env
        .get_string(&service_provider)
        .expect("Couldn't get java string!")
        .into();

    log::debug!("using sp {}", sp_input);

    // TODO pass this callback to blocking_run_client
    env.call_method(cb_object, "onStart", "()V", &[])
        .expect("failed to call Java callbacks");

    let service_provider = char_p::new(sp_input.as_str());
    crate::blocking_run_client(
        None,
        Some(service_provider.as_ref()),
        RefDynFnMut1::new(&mut |a| placeholder_startup_cb(a)),
        RefDynFnMut0::new(&mut || placeholder_shutdown_cb()),
    );
}

// hehe, I know. this is beyond disgusting
static mut STOP: fn() = || {};
static mut START: fn(a: char_p_boxed) = |a| crate::rust_free_string(a);

#[no_mangle]
pub unsafe extern "C" fn Java_net_nymtech_nyms5_NymProxy_startClient(
    _env: JNIEnv,
    _class: JClass,
    _input: JString,
) {
    // TODO: how does this work if we are not doing blocking calls?
    init_jni_logger();

    let start_cb = RefDynFnMut1::new(&mut START);
    let stop_cb = RefDynFnMut0::new(&mut STOP);

    // TODO: get the service provider from input
    let service_provider = char_p::new("DpB3cHAchJiNBQi5FrZx2csXb1mrHkpYh9Wzf8Rjsuko.ANNWrvHqMYuertHGHUrZdBntQhpzfbWekB39qez9U2Vx@2BuMSfMW3zpeAjKXyKLhmY4QW1DXurrtSPEJ6CjX3SEh");
    crate::start_client(None, Some(service_provider.as_ref()), start_cb, stop_cb);
}

#[no_mangle]
pub unsafe extern "C" fn Java_net_nymtech_nyms5_NymProxy_stopClient(
    mut env: JNIEnv,
    _class: JClass,
    cb_object: JObject,
) {
    //init_jni_logger();
    crate::stop_client();

    // fake some workload
    thread::sleep(time::Duration::from_secs(2));

    // TODO pass this callback to stop_client
    env.call_method(cb_object, "onStop", "()V", &[])
        .expect("failed to call Java callbacks");
}
