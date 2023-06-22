// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"context"
	"errors"
	"net"
	"net/http"
	"sync"
)

type RequestId = uint64

type ActiveRequests struct {
	sync.Mutex
	inner map[RequestId]*ConnectionInjector
}

func (ar *ActiveRequests) exists(id RequestId) bool {
	Debug("checking if request %d exists", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	return exists
}

func (ar *ActiveRequests) insert(id RequestId, conn *ConnectionInjector) {
	Debug("inserting request %d", id)
	ar.Lock()
	defer ar.Unlock()
	ar.inner[id] = conn
}

func (ar *ActiveRequests) remove(id RequestId) {
	Debug("removing request %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to remove active connection that doesn't exist")
	}
	delete(ar.inner, id)
}

func (ar *ActiveRequests) injectData(id RequestId, data []byte) {
	Debug("injecting data for %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to write to connection that doesn't exist")
	}
	ar.inner[id].injectedServerData <- data
}

func (ar *ActiveRequests) closeRemoteSocket(id RequestId) {
	Debug("closing remote socket for %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to close remote socket of a connection that doesn't exist")
	}
	ar.inner[id].closedRemote.Store(true)
}

func buildHttpClient(requestId RequestId) *http.Client {
	if _, exists := activeRequests.inner[requestId]; exists {
		panic("duplicate connection detected")
	}

	return &http.Client{
		Transport: &http.Transport{
			DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				Info("entered DialContext")

				if activeRequests.exists(requestId) {
					return nil, errors.New("duplicate plain connection detected")
				}

				conn, inj := NewFakeConnection(requestId)
				activeRequests.insert(requestId, &inj)

				return conn, nil
			},

			DialTLSContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				Info("entered DialTLSContext")

				if activeRequests.exists(requestId) {
					return nil, errors.New("duplicate SSL connection detected")
				}

				conn, inj := NewFakeTlsConn(requestId)
				activeRequests.insert(requestId, &inj)

				if err := conn.Handshake(); err != nil {
					return nil, err
				}

				return conn, nil
			},

			//TLSClientConfig: &tlsConfig,
			DisableKeepAlives:   true,
			MaxIdleConns:        1,
			MaxIdleConnsPerHost: 1,
			MaxConnsPerHost:     1,
		},
	}
}

func _closeRemoteSocket(requestId RequestId) any {
	activeRequests.closeRemoteSocket(requestId)
	return nil
}

func _injectServerData(requestId RequestId, data []byte) any {
	activeRequests.injectData(requestId, data)
	return nil
}

func performRequest(requestId RequestId, req *http.Request) (*http.Response, error) {
	reqClient := buildHttpClient(requestId)

	Info("Starting the request...")
	Debug("%v", *req)
	return reqClient.Do(req)
}

func _mixFetch(requestId RequestId, request *http.Request) (any, error) {
	Info("_mixFetch: start")

	resp, err := performRequest(requestId, request)
	if err != nil {
		return nil, err
	}
	Info("finished performing the request")
	Debug("response: %v", *resp)
	return intoJSResponse(resp)
}

// TODO: recreate something similar in Go:
/*
fn _new_from_init_or_input(
        url: Option<String>,
        input: Option<&Request>,
        init: Option<&RequestInitWithTypescriptType>,
    ) -> Result<WebSysRequestAdapter, MixHttpRequestError> {
        let init_default = JsValue::default();
        let mut init_or_input = &init_default;
        if let Some(init) = init {
            init_or_input = init;
        } else if let Some(input) = input {
            init_or_input = input;
        }

        // the URL will either come from an argument to this fn, or it could be a field in init that is either
        // a string or a Javascript Url object, so coerce to a string (might be empty) and parse here
        let url_from_input = get_url_field_from_some_request(input);
        let url_from_init = get_url_field_from_some_js_value(Some(init_or_input));

        // first use url, then fallback to input and finally to init
        let url_to_parse = url.or(url_from_input).or(url_from_init);

        let parsed_url = url::Url::parse(&url_to_parse.unwrap_or_default())?;

        // the target for the HTTP request is just the path component of the url
        let target = RequestTarget::new(parsed_url.path())?;

        // parse the method and default to GET if unspecified or in error
        let method_from_init = get_string_value(init_or_input, "method");
        let method_name = method_from_init.unwrap_or("GET".to_string());
        let method = Method::new(&method_name)
            .unwrap_or(Method::new("GET").expect("should always unwrap static value"));

        let headers = get_object_value(init_or_input, "headers");
        let body = get_object_value(init_or_input, "body");

        // possibly support `navigate` in the future?
        let _mode = get_string_value(init_or_input, "mode");

        // currently unsupported, could possibly get the credentials (e.g. basic auth)
        // from the https://developer.mozilla.org/en-US/docs/Web/API/Navigator/credentials prop
        let _credentials = get_string_value(init_or_input, "credentials");

        // currently this is unsupported, however, we could consider using the Cache API:
        // https://developer.mozilla.org/en-US/docs/Web/API/Cache/match
        let _cache = get_string_value(init_or_input, "cache");

        // currently this is unsupported, relatively easy the implement
        let _redirect = get_string_value(init_or_input, "redirect");

        // do we want to pass on this information?
        let _referrer = get_string_value(init_or_input, "referrer");
        let _referrer_policy = get_string_value(init_or_input, "referrerPolicy");

        // should we check the integrity of the return data?
        let _integrity = get_string_value(init_or_input, "integrity");

        // this might be a way to signal to keep the other side of the SOCKS5 client open
        let _keepalive = get_boolean_value(init_or_input, "keepalive");

        // not implemented, not possible to cancel
        let _signal = get_object_value(init_or_input, "signal");

        // not implemented
        let _priority = get_string_value(init_or_input, "priority");

        let byte_serialized_body = BodyFromJsValue::new(&body);

        let mut request =
            HttpCodecRequest::new(method, target, HttpVersion::V1_1, byte_serialized_body.body);

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

        // check if the caller has set the content type, otherwise, set it from the body if possible
        if !request_headers.fields().any(|f| f.name() == "Content-Type") {
            if let Some(mime_type) = byte_serialized_body.mime_type {
                request_headers.add_field(HeaderField::new("Content-Type", &mime_type)?);
            }
        }

        Ok(WebSysRequestAdapter {
            target: remote_address_from_url(&parsed_url)?,
            request,
        })
    }


#[derive(Default, Debug)]
struct BodyFromJsValue {
    pub(crate) body: Vec<u8>,
    pub(crate) mime_type: Option<String>,
}

impl BodyFromJsValue {
    pub fn new(js_value: &Option<JsValue>) -> Self {
        match js_value {
            None => BodyFromJsValue::default(),
            Some(val) => {
                // for string types, convert them into UTF-8 byte arrays
                if val.is_string() {
                    return Self::string_plain(val);
                }

                // try get the constructor function name (the class name) for polymorphic fetch body types
                match get_class_name_or_type(val) {
                    Some(class_name_or_type) => match class_name_or_type.as_str() {
                        "FormData" => Self::form_data_to_vec(val),
                        "Uint8Array" => Self::array_to_vec(val),
                        "Array" => Self::array_to_vec(val),
                        &_ => BodyFromJsValue::default(),
                    },
                    None => BodyFromJsValue::default(),
                }
            }
        }
    }

    fn string_plain(js_value: &JsValue) -> BodyFromJsValue {
        BodyFromJsValue {
            body: js_value.as_string().unwrap_or_default().into_bytes(),
            mime_type: Some("text/plain".to_string()),
        }
    }

    fn array_to_vec(js_value: &JsValue) -> BodyFromJsValue {
        let array = js_sys::Uint8Array::new(js_value);
        BodyFromJsValue {
            body: array.to_vec(),
            mime_type: Some("application/octet-stream".to_string()),
        }
    }

    fn form_data_to_vec(js_value: &JsValue) -> BodyFromJsValue {
        let mut serializer = form_urlencoded::Serializer::new(String::new());

        let form = FormDataWithKeys::attach(js_value);

        for form_key in form.keys().into_iter().flatten() {
            if let Some(form_key) = form_key.as_string() {
                if let Some(val) = form.get(&form_key).as_string() {
                    serializer.append_pair(&form_key, &val);
                }
            }
        }

        // same as `Object.keys(headers).forEach(...)`
        if let Ok(keys) = js_sys::Reflect::own_keys(js_value) {
            for key in keys.iter() {
                if let Some(key) = key.as_string() {
                    if let Some(val) = get_string_value(js_value, &key) {
                        serializer.append_pair(&key, &val);
                    }
                }
            }
        }

        BodyFromJsValue {
            body: serializer.finish().into_bytes(),
            mime_type: Some("application/x-www-form-urlencoded".to_string()),
        }
    }
}

*/
