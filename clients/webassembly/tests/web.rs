//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

use nym_client_wasm::mix_fetch::mix_http_requests::{
    FetchToHttpRequestString, RequestInitWithTypescriptType,
};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn http_get_ok() {
    let fetch_to_http_request_string = FetchToHttpRequestString::new();
    let init = RequestInitWithTypescriptType::new();
    let res = fetch_to_http_request_string
        .fetch_with_str_and_init("https://nymtech.net".to_string(), &init)
        .unwrap_or("".to_string());

    let expected = r#"GET / HTTP/1.1
Host: https://nymtech.net
Content-Length: 0

"#
    .replace("\n", "\r\n");
    assert_eq!(expected, res);
}

#[wasm_bindgen_test]
fn http_get_with_headers_ok() {
    let init = RequestInitWithTypescriptType::from_json(
        r#"{
    "headers": {
       "Accepts": "application/json",
       "Content-Type": "application/json"
    }
}"#,
    );

    let fetch_to_http_request_string = FetchToHttpRequestString::new();

    let res = fetch_to_http_request_string
        .fetch_with_str_and_init("https://nymtech.net".to_string(), &init)
        .unwrap_or("".to_string());

    let expected = r#"GET / HTTP/1.1
Host: https://nymtech.net
Accepts: application/json
Content-Type: application/json
Content-Length: 0

"#
    .replace("\n", "\r\n");
    assert_eq!(expected, res);
}

#[wasm_bindgen_test]
fn http_post_json_string_ok() {
    // make an RequestInit struct with placeholder for the body
    let init = RequestInitWithTypescriptType::from_json(
        r#"{
    "body": "replace_me",
    "method": "POST",
    "headers": {
        "Accepts": "application/json",
        "Content-Type": "application/json"
    }
}"#,
    );

    // make the body a JsValue
    let json = JsValue::from_str(r#"{ "foo": 1, "bar": 2 }"#);

    // replace the placeholders
    js_sys::Reflect::set(&init, &JsValue::from_str("body"), &json).expect("can set body value");

    let fetch_to_http_request_string = FetchToHttpRequestString::new();

    let res = fetch_to_http_request_string
        .fetch_with_str_and_init(
            "https://validator.nymtech.net/api/v1/baz".to_string(),
            &init,
        )
        .unwrap_or("".to_string());

    let expected = r#"POST /api/v1/baz HTTP/1.1
Host: https://validator.nymtech.net
Accepts: application/json
Content-Type: application/json
Content-Length: 22

{ "foo": 1, "bar": 2 }"#
        .replace("\n", "\r\n");
    assert_eq!(expected, res);
}

#[wasm_bindgen_test]
fn http_post_json_uint8array_ok() {
    // make an RequestInit struct with placeholder for the body
    let init = RequestInitWithTypescriptType::from_json(
        r#"{
    "body": "replace_me",
    "method": "POST",
    "headers": {
        "Accepts": "application/json",
        "Content-Type": "application/json"
    }
}"#,
    );

    let byte_array =
        js_sys::eval("new Uint8Array([1,2,3,4,5,6])").expect("JS executes and creates array");

    // replace the placeholders
    js_sys::Reflect::set(&init, &JsValue::from_str("body"), &byte_array)
        .expect("can set body value");

    let fetch_to_http_request_string = FetchToHttpRequestString::new();

    let res = fetch_to_http_request_string
        .fetch_with_str_and_init(
            "https://validator.nymtech.net/api/v1/baz".to_string(),
            &init,
        )
        .unwrap_or("".to_string());

    let expected = format!(
        r#"POST /api/v1/baz HTTP/1.1
Host: https://validator.nymtech.net
Accepts: application/json
Content-Type: application/json
Content-Length: 6

{}"#,
        "\u{1}\u{2}\u{3}\u{4}\u{5}\u{6}"
    )
    .replace("\n", "\r\n");
    assert_eq!(expected, res);
}

#[wasm_bindgen_test]
fn http_post_basic_form_params_ok() {
    // make an RequestInit struct with placeholder for the body
    let init = RequestInitWithTypescriptType::from_json(
        r#"{
    "body": "replace_me",
    "method": "POST",
    "headers": {
        "Accepts": "application/json"
    }
}"#,
    );

    let form_data = web_sys::FormData::new().expect("FormData object can be created");
    form_data
        .append_with_str("field1", "value1")
        .expect("should add");
    form_data
        .append_with_str("field2", "this is a value with 👌 data in it")
        .expect("should add");

    // replace the placeholders
    js_sys::Reflect::set(&init, &JsValue::from_str("body"), &form_data)
        .expect("can set body value");

    let fetch_to_http_request_string = FetchToHttpRequestString::new();

    let res = fetch_to_http_request_string
        .fetch_with_str_and_init(
            "https://validator.nymtech.net/api/v1/baz".to_string(),
            &init,
        )
        .unwrap_or("".to_string());

    let expected = r#"POST /api/v1/baz HTTP/1.1
Host: https://validator.nymtech.net
Accepts: application/json
Content-Type: application/x-www-form-urlencoded
Content-Length: 65

field1=value1&field2=this+is+a+value+with+%F0%9F%91%8C+data+in+it"#
        .replace("\n", "\r\n");
    assert_eq!(expected, res);
}
