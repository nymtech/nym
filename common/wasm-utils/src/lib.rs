// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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

/// Maps provided `Result`'s inner values into a pair of `JsValue` that can be returned
/// inside a promise (and in particular from inside `future_to_promise`)
pub fn into_promise_result<T, E>(res: Result<T, E>) -> Result<JsValue, JsValue>
where
    T: Into<JsValue>,
    E: Into<JsValue>,
{
    res.map(Into::into).map_err(Into::into)
}

pub fn map_promise_err<T, E>(res: Result<T, E>) -> Result<T, JsValue>
where
    E: Into<JsValue>,
{
    res.map_err(Into::into)
}

pub trait PromisableResult {
    fn into_promise_result(self) -> Result<JsValue, JsValue>;
}

// this should probably get renamed : )
pub trait PromisableResultError {
    type Ok;

    fn map_promise_err(self) -> Result<Self::Ok, JsValue>;
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

impl<T, E> PromisableResultError for Result<T, E>
where
    E: Into<JsValue>,
{
    type Ok = T;

    fn map_promise_err(self) -> Result<T, JsValue> {
        map_promise_err(self)
    }
}

#[macro_export]
macro_rules! check_promise_result {
    ( $x:expr ) => {
        match $crate::PromisableResultError::map_promise_err($x) {
            Ok(r) => r,
            Err(err) => return js_sys::Promise::reject(&err),
        }
    };
}
