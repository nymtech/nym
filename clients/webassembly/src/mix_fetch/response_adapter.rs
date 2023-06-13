// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use wasm_bindgen::JsValue;

pub(crate) trait FetchResponse {
    fn try_into_fetch_response(self) -> Result<web_sys::Response, JsValue>;
}

impl FetchResponse for httpcodec::Response<Vec<u8>> {
    fn try_into_fetch_response(mut self) -> Result<web_sys::Response, JsValue> {
        // TODO: see if we can add it directly somehow
        let header = self.header();
        let mut headers: HashMap<&str, &str> = HashMap::new();
        for header_field in header.fields() {
            headers.insert(header_field.name(), header_field.value());
        }
        let headers = serde_wasm_bindgen::to_value(&headers)
            .expect("unexpected headers serialization failure!");
        let status_code = self.status_code().as_u16();

        let body = self.body_mut();
        let mut init = web_sys::ResponseInit::new();
        init.status(status_code);
        init.headers(&headers);
        // init.headers(&JsValue::from_str(&self.header().to_string()));

        web_sys::Response::new_with_opt_u8_array_and_init(Some(body), &init)
    }
}
