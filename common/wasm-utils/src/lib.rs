// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::TryFutureExt;
use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

#[cfg(feature = "websocket")]
pub mod websocket;

// will cause messages to be written as if console.log("...") was called
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => ($crate::log(&format_args!($($t)*).to_string()))
}

// will cause messages to be written as if console.warn("...") was called
#[macro_export]
macro_rules! console_warn {
    ($($t:tt)*) => ($crate::warn(&format_args!($($t)*).to_string()))
}

// will cause messages to be written as if console.error("...") was called
#[macro_export]
macro_rules! console_error {
    ($($t:tt)*) => ($crate::error(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn warn(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn error(s: &str);
}

#[cfg(feature = "sleep")]
pub async fn sleep(ms: i32) -> Result<(), JsValue> {
    let promise = Promise::new(&mut |yes, _| {
        let win = web_sys::window().expect("no window available!");
        win.set_timeout_with_callback_and_timeout_and_arguments_0(&yes, ms)
            .unwrap();
    });
    let js_fut = JsFuture::from(promise);
    js_fut.await?;
    Ok(())
}

/// A helper that construct a `JsValue` containing an error with the provided message.
pub fn simple_js_error<S: AsRef<str>>(message: S) -> JsValue {
    let js_error = js_sys::Error::new(message.as_ref());
    JsValue::from(js_error)
}

#[macro_export]
macro_rules! js_error {
    ($($t:tt)*) => {{
        let js_error = js_sys::Error::new(&format!($($t)*));
        wasm_bindgen::JsValue::from(js_error)
    }}
}

pub fn into_promise_result<T, E>(res: Result<T, E>) -> Result<JsValue, JsValue>
where
    T: Into<JsValue>,
    E: Into<JsValue>,
{
    res.map(Into::into).map_err(Into::into)
}

pub trait PromisableResult {
    fn into_promise_result(self) -> Result<JsValue, JsValue>;
}

impl<T, E> PromisableResult for Result<T, E>
where
    T: Into<JsValue>,
    E: Into<JsValue>,
{
    fn into_promise_result(self) -> Result<JsValue, JsValue> {
        into_promise_result(self)
    }
}
