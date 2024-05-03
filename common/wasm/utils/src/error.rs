// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use js_sys::Promise;
use wasm_bindgen::JsValue;

#[macro_export]
macro_rules! wasm_error {
    ($struct:ident) => {
        impl $struct {
            pub fn into_rejected_promise(self) -> js_sys::Promise {
                self.into()
            }
        }

        impl From<$struct> for wasm_bindgen::JsValue {
            fn from(value: $struct) -> Self {
                $crate::error::simple_js_error(value.to_string())
            }
        }

        impl From<$struct> for js_sys::Promise {
            fn from(value: $struct) -> Self {
                js_sys::Promise::reject(&value.into())
            }
        }
    };
}

/// A helper that constructs a `Promise` containing a wrapper JS error.
pub fn simple_rejected_promise<S: AsRef<str>>(err_message: S) -> Promise {
    let err = simple_js_error(err_message);
    Promise::reject(&err)
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
        match $crate::error::PromisableResultError::map_promise_err($x) {
            Ok(r) => r,
            Err(err) => return js_sys::Promise::reject(&err),
        }
    };
}
