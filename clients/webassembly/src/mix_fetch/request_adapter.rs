use httpcodec::{HeaderField, HttpVersion, Method, Request as HttpCodecRequest, RequestTarget};
use nym_http_requests::error::MixHttpRequestError;
use std::any::Any;
use wasm_bindgen::JsValue;
use wasm_utils::console_log;
use web_sys::Request;

use crate::mix_fetch::mix_http_requests::RequestInitWithTypescriptType;

pub(crate) struct WebSysRequestAdapter {
    request: HttpCodecRequest<Vec<u8>>,
}

impl WebSysRequestAdapter {
    pub(crate) fn new_from_string(
        input: &str,
    ) -> Result<WebSysRequestAdapter, MixHttpRequestError> {
        let url = url::Url::parse(input)?;
        let target = RequestTarget::new(url.path())?;

        let mut request = HttpCodecRequest::new(
            Method::new("GET").unwrap(),
            target,
            HttpVersion::V1_1,
            b"".to_vec(),
        );

        let mut request_headers = request.header_mut();

        let origin = url.origin().unicode_serialization();
        request_headers.add_field(HeaderField::new("Host", &origin)?);

        Ok(WebSysRequestAdapter { request })
    }

    pub(crate) fn new_from_request(
        _request: &Request,
    ) -> Result<WebSysRequestAdapter, MixHttpRequestError> {
        let request = HttpCodecRequest::new(
            Method::new("GET").unwrap(),
            RequestTarget::new("/.wellknown/wallet/validators.json").unwrap(),
            HttpVersion::V1_1,
            b"".to_vec(),
        );
        Ok(WebSysRequestAdapter { request })
    }

    pub(crate) fn new_from_init_or_input(
        url: Option<String>,
        input: Option<&Request>,
        init: &RequestInitWithTypescriptType,
    ) -> Result<WebSysRequestAdapter, MixHttpRequestError> {
        // the URL will either come from an argument to this fn, or it could be a field in init that is either
        // a string or a Javascript Url object, so coerce to a string (might be empty) and parse here
        let url_from_input = get_url_field_from_some_request(input);
        let url_from_init = get_url_field_from_some_js_value(Some(init));

        // first use url, then fallback to input and finally to init
        let url_to_parse = url.or(url_from_input).or(url_from_init);

        let parsed_url = url::Url::parse(&url_to_parse.unwrap_or_default())?;

        // the target for the HTTP request is just the path component of the url
        let target = RequestTarget::new(parsed_url.path())?;

        // parse the method and default to GET if unspecified or in error
        let method_from_init = get_string_value(init, "method");
        let method_name = method_from_init.unwrap_or("GET".to_string());
        let method = Method::new(&method_name)
            .unwrap_or(Method::new("GET").expect("should always unwrap static value"));

        let headers = get_object_value(init, "headers");
        let body = get_object_value(init, "body");
        let _mode = get_string_value(init, "mode");
        let _credentials = get_string_value(init, "credentials");
        let _cache = get_string_value(init, "cache");
        let _redirect = get_string_value(init, "redirect");
        let _referrer = get_string_value(init, "referrer");
        let _referrer_policy = get_string_value(init, "referrerPolicy");
        let _integrity = get_string_value(init, "integrity");
        let _keepalive = get_boolean_value(init, "keepalive");
        let _signal = get_object_value(init, "signal");
        let _priority = get_string_value(init, "priority");

        let body = body_from_js_value(&body);

        let mut request = HttpCodecRequest::new(method, target, HttpVersion::V1_1, body);

        let mut request_headers = request.header_mut();

        // the Host header will be something like `https://example.com:3000` or `https://example.com`
        // when not present it will be the string with value `null`
        let origin = parsed_url.origin().unicode_serialization();
        request_headers.add_field(HeaderField::new("Host", &origin)?);

        // add headers
        if let Some(h) = headers {
            // same as `Object.keys(headers).forEach(...)`
            if let Ok(keys) = js_sys::Reflect::own_keys(&h) {
                for key in keys.iter() {
                    if let Some(key) = key.as_string() {
                        if let Some(val) = get_string_value(&h, &key) {
                            if let Ok(header) = HeaderField::new(&key, &val) {
                                request_headers.add_field(header);
                            }
                        }
                    }
                }
            }
        }

        Ok(WebSysRequestAdapter { request })
    }

    pub(crate) fn http_codec_request(self) -> HttpCodecRequest<Vec<u8>> {
        self.request
    }
}

fn get_string_value(js_value: &JsValue, key: &str) -> Option<String> {
    match js_sys::Reflect::get(js_value, &JsValue::from(key)) {
        Ok(val) => val.as_string(),
        Err(_) => None,
    }
}

fn get_boolean_value(js_value: &JsValue, key: &str) -> Option<bool> {
    match js_sys::Reflect::get(js_value, &JsValue::from(key)) {
        Ok(val) => val.as_bool(),
        Err(_) => None,
    }
}

fn get_object_value(js_value: &JsValue, key: &str) -> Option<JsValue> {
    js_sys::Reflect::get(js_value, &JsValue::from(key)).ok()
}

fn get_url_field_from_some_js_value(js_value: Option<&JsValue>) -> Option<String> {
    js_value.and_then(|x| get_object_value(x, "url").map(|x| x.as_string().unwrap_or_default()))
}

fn get_url_field_from_some_request(request: Option<&Request>) -> Option<String> {
    request.and_then(|x| get_object_value(x, "url").map(|x| x.as_string().unwrap_or_default()))
}

fn body_from_js_value(js_value: &Option<JsValue>) -> Vec<u8> {
    match js_value {
        None => vec![],
        Some(val) => {
            if val.is_string() {
                return val.as_string().unwrap_or_default().into_bytes();
            }

            let proto = js_sys::Reflect::get_prototype_of(val);
            console_log!("🔫🔫🔫val = {:?}", val);
            if let Ok(p2) = proto {
                let pfn = p2.constructor();
                console_log!("🔫🔫🔫constructor = {:?}", pfn.name());
            }

            // TODO: this is nasty, but can't find any other way
            let array = js_sys::Uint8Array::new(val);
            array.to_vec()
        }
    }
}
