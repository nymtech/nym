// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"context"
	"errors"
	"fmt"
	"net"
	"net/http"
	"sync"
	"syscall/js"
	"time"
)

type RequestId = uint64

type ActiveRequests struct {
	sync.Mutex
	inner map[RequestId]*ActiveRequest
}

func (ar *ActiveRequests) exists(id RequestId) bool {
	Debug("checking if request %d exists", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	return exists
}

func (ar *ActiveRequests) insert(id RequestId, inj ConnectionInjector) {
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if exists {
		panic("attempted to overwrite active connection")
	}
	ar.inner[id] = &ActiveRequest{injector: inj}

	//
	//if !exists {
	//	ar.inner[id] = &ActiveRequest{
	//		target:           target,
	//		expectedRedirect: nil,
	//		injector:         inj,
	//	}
	//} else {
	//	if existing.expectedRedirect == nil {
	//		panic("attempted to overwrite active connection: no redirect set")
	//	} else if existing.expectedRedirect.String() != target.String() {
	//		// TODO: is the string comparison really the way to do it?
	//		println(existing.expectedRedirect.String())
	//		println(target.String())
	//		panic("attempted to overwrite active connection: mismatched redirect")
	//	} else {
	//		existing.injector.redirected.Store(true)
	//		go func() {
	//			// TODO: timeout to make sure the connection got closed and cleaned up
	//		}()
	//
	//		existing.target = target
	//		existing.expectedRedirect = nil
	//		existing.injector = inj
	//	}
	//}
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
	ar.inner[id].injector.serverData <- data
}

//func (ar *ActiveRequests) setRedirect(id RequestId, redirect *url.URL) {
//	Debug("setting redirect for %d to %s", id, redirect.String())
//	ar.Lock()
//	defer ar.Unlock()
//	_, exists := ar.inner[id]
//	if !exists {
//		panic("attempted to set redirect on a connection that doesn't exist")
//	}
//	ar.inner[id].expectedRedirect = redirect
//}

func (ar *ActiveRequests) closeRemoteSocket(id RequestId) {
	Debug("closing remote socket for %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to close remote socket of a connection that doesn't exist")
	}
	ar.inner[id].injector.remoteClosed.Store(true)
}

type ActiveRequest struct {
	injector ConnectionInjector
}

func buildHttpClient(requestId RequestId, redirect Redirect) *http.Client {
	return nil
	//if _, exists := activeRequests.inner[requestId]; exists {
	//	panic("duplicate connection detected")
	//}
	//
	//return &http.Client{
	//	CheckRedirect: func(req *http.Request, via []*http.Request) error {
	//		Debug("attempting to perform redirection to %s with our policy set to '%s'", req.URL.String(), redirect)
	//		redirectionChain := ""
	//		for i := 0; i < len(via); i++ {
	//			redirectionChain += fmt.Sprintf("%s -> ", via[i].URL.String())
	//		}
	//		redirectionChain += fmt.Sprintf("[%s]", req.URL.String())
	//		Debug("redirection chain: %s", redirectionChain)
	//
	//		if redirect == REQUEST_REDIRECT_MANUAL {
	//			Error("unimplemented '%s' redirect", redirect)
	//			return http.ErrUseLastResponse
	//		}
	//		// TODO: is this actually the correct use of the `error` redirect?
	//		if redirect == REQUEST_REDIRECT_ERROR {
	//			return errors.New("encountered redirect")
	//		}
	//		if redirect == REQUEST_REDIRECT_FOLLOW {
	//			Debug("will perform redirection")
	//			// TODO: either here or in actual `Dial` we need to call rust to start the socks5 all over again
	//			// but this will call `goWasmMixFetch`... do we want that?
	//			return nil
	//		}
	//		// if this was rust that had proper enums and match statements,
	//		// we could have guaranteed that at compile time...
	//		panic("unreachable")
	//	},
	//
	//	Transport: &http.Transport{
	//		DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
	//			Info("dialing plain connection to %s", addr)
	//
	//			conn, inj := NewFakeConnection(requestId, addr)
	//
	//			// if this doesn't work here, everything will blow up anyway
	//			parsedAddr, err := url.Parse(addr)
	//			if err != nil {
	//				return nil, err
	//			}
	//			activeRequests.insert(requestId, inj, parsedAddr)
	//
	//			return conn, nil
	//		},
	//
	//		DialTLSContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
	//			Info("dialing TLS connection to %s", addr)
	//
	//			conn, inj := NewFakeTlsConn(requestId, addr)
	//
	//			// if this doesn't work here, everything will blow up anyway
	//			parsedAddr, err := url.Parse(addr)
	//			if err != nil {
	//				return nil, err
	//			}
	//			activeRequests.insert(requestId, inj, parsedAddr)
	//
	//			if err := conn.Handshake(); err != nil {
	//				return nil, err
	//			}
	//
	//			return conn, nil
	//		},
	//
	//		//TLSClientConfig: &tlsConfig,
	//		DisableKeepAlives:   true,
	//		MaxIdleConns:        1,
	//		MaxIdleConnsPerHost: 1,
	//		MaxConnsPerHost:     1,
	//	},
	//}
}

func startMixnetConnection(address string) (RequestId, error) {
	Debug("attempting to start mixnet connection for %s", address)

	requestPromise := js.Global().Call("start_new_mixnet_connection", address)
	Debug("promise: %v", requestPromise)
	rawRequestId, errRes := await(requestPromise)
	Debug("results: %v and %v", rawRequestId, errRes)

	if errRes != nil {
		Debug("error: %v", errRes)
		panic("todo err")
		//return nil, err
	}

	if len(rawRequestId) != 1 {
		panic("todo len")
	}

	requestId, err := parseRequestId(rawRequestId[0])
	if err != nil {
		return 0, err
	}
	if activeRequests.exists(requestId) {
		panic("todo duplicate")
		//panic("duplicate connection detected")
	}

	return requestId, nil

}

func buildHttpClient2(redirect Redirect) *http.Client {
	//if _, exists := activeRequests.inner[requestId]; exists {
	//	panic("duplicate connection detected")
	//}

	return &http.Client{
		CheckRedirect: func(req *http.Request, via []*http.Request) error {
			Debug("attempting to perform redirection to %s with our policy set to '%s'", req.URL.String(), redirect)
			redirectionChain := ""
			for i := 0; i < len(via); i++ {
				redirectionChain += fmt.Sprintf("%s -> ", via[i].URL.String())
			}
			redirectionChain += fmt.Sprintf("[%s]", req.URL.String())
			Debug("redirection chain: %s", redirectionChain)

			if redirect == REQUEST_REDIRECT_MANUAL {
				Error("unimplemented '%s' redirect", redirect)
				return http.ErrUseLastResponse
			}
			// TODO: is this actually the correct use of the `error` redirect?
			if redirect == REQUEST_REDIRECT_ERROR {
				return errors.New("encountered redirect")
			}
			if redirect == REQUEST_REDIRECT_FOLLOW {
				Debug("will perform redirection")
				return nil
			}
			// if this was rust that had proper enums and match statements,
			// we could have guaranteed that at compile time...
			panic("unreachable")
		},

		Transport: &http.Transport{
			DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				Info("dialing plain connection to %s", addr)

				requestId, err := startMixnetConnection(addr)
				if err != nil {
					return nil, err
				}

				conn, inj := NewFakeConnection(requestId, addr)
				activeRequests.insert(requestId, inj)

				return conn, nil
			},

			DialTLSContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				Info("dialing TLS connection to %s", addr)

				requestId, err := startMixnetConnection(addr)
				if err != nil {
					return nil, err
				}

				conn, inj := NewFakeTlsConn(requestId, addr)
				activeRequests.insert(requestId, inj)

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

func buildHttpClient3(redirect Redirect) *http.Client {
	//if _, exists := activeRequests.inner[requestId]; exists {
	//	panic("duplicate connection detected")
	//}

	return &http.Client{
		CheckRedirect: func(req *http.Request, via []*http.Request) error {
			Debug("attempting to perform redirection to %s with our policy set to '%s'", req.URL.String(), redirect)
			redirectionChain := ""
			for i := 0; i < len(via); i++ {
				redirectionChain += fmt.Sprintf("%s -> ", via[i].URL.String())
			}
			redirectionChain += fmt.Sprintf("[%s]", req.URL.String())
			Debug("redirection chain: %s", redirectionChain)

			switch redirect {
			case REQUEST_REDIRECT_MANUAL:
				Error("unimplemented '%s' redirect", redirect)
				return http.ErrUseLastResponse
			case REQUEST_REDIRECT_ERROR:
				return errors.New("encountered redirect")
			case REQUEST_REDIRECT_FOLLOW:
				Debug("will perform redirection")
				return nil
			}

			// if this was rust that had proper enums and match statements,
			// we could have guaranteed that at compile time...
			panic("unreachable")
		},

		Transport: &http.Transport{
			DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				Info("dialing plain connection to %s", addr)

				requestId, err := rsStartNewMixnetRequest(addr)
				if err != nil {
					return nil, err
				}

				conn, inj := NewFakeConnection(requestId, addr)
				activeRequests.insert(requestId, inj)

				return conn, nil
			},

			DialTLSContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				Info("dialing TLS connection to %s", addr)

				requestId, err := rsStartNewMixnetRequest(addr)
				if err != nil {
					return nil, err
				}

				conn, inj := NewFakeTlsConn(requestId, addr)
				activeRequests.insert(requestId, inj)

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

func performRequest(requestId RequestId, req *ParsedRequest) (*http.Response, error) {
	reqClient := buildHttpClient(requestId, req.redirect)

	Info("Starting the request...")
	Debug("%s: %v", req.redirect, *req.request)
	return reqClient.Do(req.request)
}

func performRequest2(req *ParsedRequest) (*http.Response, error) {
	reqClient := buildHttpClient2(req.redirect)

	Info("Starting the request...")
	Debug("%s: %v", req.redirect, *req.request)
	return reqClient.Do(req.request)
}

func performRequest3(req *ParsedRequest) (*http.Response, error) {
	reqClient := buildHttpClient3(req.redirect)

	Info("Starting the request...")
	Debug("%s: %v", req.redirect, *req.request)
	return reqClient.Do(req.request)
}

func _mixFetch(requestId RequestId, request *ParsedRequest) (any, error) {
	Info("_mixFetch: start")

	resp, err := performRequest(requestId, request)
	if err != nil {
		return nil, err
	}
	Info("finished performing the request")
	Debug("response: %v", *resp)
	return intoJSResponse(resp)
}

func _mixFetch2(request *ParsedRequest) (any, error) {
	Info("_mixFetch: start")

	resp, err := performRequest2(request)
	if err != nil {
		return nil, err
	}
	Info("finished performing the request")
	Debug("response: %v", *resp)
	return intoJSResponse(resp)
}

func _mixFetch3(request *ParsedRequest) (any, error) {
	Info("_mixFetch: start")

	resCh := make(chan *http.Response)
	errCh := make(chan error)
	go func(resCh chan *http.Response, errCh chan error) {
		resp, err := performRequest3(request)
		if err != nil {
			errCh <- err
		} else {
			resCh <- resp
		}
	}(resCh, errCh)

	select {
	case res := <-resCh:
		Info("finished performing the request")
		Debug("response: %v", *res)
		return intoJSResponse(res)
	case err := <-errCh:
		Warn("request failure: %v", err)
		return nil, err
	case <-time.After(requestTimeout):
		// TODO: cancel stuff here.... somehow...
		Warn("request has timed out")
		return nil, errors.New("request timeout")
	}
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
