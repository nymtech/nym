use httpcodec::Request as HttpCodecRequest;
use js_sys::Uint8Array;
use nym_service_providers_common::interface::Serializable;
use nym_socks5_requests::Socks5ProtocolVersion;
use wasm_bindgen::prelude::*;
use web_sys::Request;

use crate::mix_fetch::request_adapter::WebSysRequestAdapter;

#[wasm_bindgen]
extern "C" {
    # [wasm_bindgen (extends = :: js_sys :: Object , js_name = RequestInit, typescript_type = "RequestInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `RequestInit` dictionary."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    pub type RequestInitWithTypescriptType;
}

impl RequestInitWithTypescriptType {
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret
    }

    pub fn from_json(json: &str) -> Self {
        let js_value = js_sys::JSON::parse(json).expect("can parse json");

        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(js_value);
        ret
    }
}

#[wasm_bindgen]
#[derive(Default)]
pub struct FetchToMixnetRequest {}

#[wasm_bindgen]
impl FetchToMixnetRequest {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        FetchToMixnetRequest::default()
    }

    pub fn fetch_with_request(&self, input: &Request) -> Result<Uint8Array, JsError> {
        http_request_to_mixnet_request_byte_array(
            WebSysRequestAdapter::new_from_request(input)?.http_codec_request(),
        )
    }

    pub fn fetch_with_str(&self, input: &str) -> Result<Uint8Array, JsError> {
        http_request_to_mixnet_request_byte_array(
            WebSysRequestAdapter::new_from_string(input)?.http_codec_request(),
        )
    }

    pub fn fetch_with_request_and_init(
        &self,
        input: &Request,
        init: &RequestInitWithTypescriptType,
    ) -> Result<Uint8Array, JsError> {
        http_request_to_mixnet_request_byte_array(
            WebSysRequestAdapter::new_from_init_or_input(None, Some(input), init)?
                .http_codec_request(),
        )
    }

    pub fn fetch_with_str_and_init(
        &self,
        input: String,
        init: &RequestInitWithTypescriptType,
    ) -> Result<Uint8Array, JsError> {
        http_request_to_mixnet_request_byte_array(
            WebSysRequestAdapter::new_from_init_or_input(Some(input), None, init)?
                .http_codec_request(),
        )
    }
}

#[wasm_bindgen]
#[derive(Default)]
pub struct FetchToHttpRequestString {}

#[wasm_bindgen]
impl FetchToHttpRequestString {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        FetchToHttpRequestString::default()
    }

    pub fn fetch_with_request(&self, input: &Request) -> Result<String, JsError> {
        http_request_to_string(WebSysRequestAdapter::new_from_request(input)?.http_codec_request())
    }

    pub fn fetch_with_str(&self, input: &str) -> Result<String, JsError> {
        http_request_to_string(WebSysRequestAdapter::new_from_string(input)?.http_codec_request())
    }

    pub fn fetch_with_request_and_init(
        &self,
        input: &Request,
        init: &RequestInitWithTypescriptType,
    ) -> Result<String, JsError> {
        http_request_to_string(
            WebSysRequestAdapter::new_from_init_or_input(None, Some(input), init)?
                .http_codec_request(),
        )
    }

    pub fn fetch_with_str_and_init(
        &self,
        input: String,
        init: &RequestInitWithTypescriptType,
    ) -> Result<String, JsError> {
        http_request_to_string(
            WebSysRequestAdapter::new_from_init_or_input(Some(input), None, init)?
                .http_codec_request(),
        )
    }
}

fn http_request_to_string(req: HttpCodecRequest<Vec<u8>>) -> Result<String, JsError> {
    Ok(nym_http_requests::encode_http_request_as_string(req)?)
}

fn http_request_to_mixnet_request_byte_array(
    req: HttpCodecRequest<Vec<u8>>,
) -> Result<Uint8Array, JsError> {
    let mixnet_req = nym_http_requests::encode_http_request_as_socks_request(
        Socks5ProtocolVersion::Versioned(5),
        0u64,
        req,
        None,
        true,
    )?;
    let buf = mixnet_req.into_bytes();
    Ok(buf.as_slice().into())
}
