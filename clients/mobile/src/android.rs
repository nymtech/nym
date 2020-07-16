#![cfg(target_os = "android")]
#![allow(non_snake_case)]

use crate::commands;
use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;
use std::ffi::CString;
use std::env;

#[no_mangle]
pub extern "system" fn Java_nym_mobile_nym_1mobile_MainActivity_init(
  env: JNIEnv,
  _: JClass,
  id: JString,
) {

    env::set_var("RUST_BACKTRACE", "full");
    env::set_var("RUST_APP_LOG", "debug");

  let id: String = env
    .get_string(id)
    .expect("Couldn't get Id!")
    .into();
  
    println!("Id: {}", id);
    commands::init::execute(id);

}

#[no_mangle]
pub extern "system" fn Java_nym_mobile_MainActivity_run(
  env: JNIEnv,
  _: JClass,
  id: JString,
  host: JString,
) {

    env::set_var("RUST_BACKTRACE", "full");
    env::set_var("RUST_APP_LOG", "debug");

  let id: String = env
    .get_string(id)
    .expect("Couldn't get Id!")
    .into();

  let host: String = env
    .get_string(host)
    .expect("Couldn't get Host!")
    .into();
  
    println!("Id: {}", id);
    commands::run::execute(id, host);


}

