// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use]
use crate::{console_log, console_error};
use crate::websocket::state::State;
use futures::{Future, Sink, Stream};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use tungstenite::{Error as WsError, Message as WsMessage};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket}; // use tungstenite Message and Error types for easier compatibility with `ClientHandshake`

mod state;

pub struct WsStream {
    socket: web_sys::WebSocket,

    message_queue: Rc<RefCell<VecDeque<WsMessage>>>,

    /// Waker of a task wanting to read incoming messages.
    stream_waker: Rc<RefCell<Option<Waker>>>,

    /// Waker of a task wanting to write to the sink.
    sink_waker: Rc<RefCell<Option<Waker>>>,

    // The callback closures. We need to store them as they will invalidate their
    // corresponding JS callback whenever they are dropped, so if we were to
    // normally return from `new` then our registered closures will
    // raise an exception when invoked.
    _on_open: Closure<dyn FnMut(JsValue)>,
    _on_error: Closure<dyn FnMut(ErrorEvent)>,
    // _on_close: Closure<dyn FnMut(CloseEvent)>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

// Unfortunately this can't be cleanly done with TryFrom/TryInto traits as both are foreign types
fn try_message_event_into_ws_message(msg_event: MessageEvent) -> Result<WsMessage, WsError> {
    match msg_event.data() {
        buf if buf.is_instance_of::<js_sys::ArrayBuffer>() => {
            let array = js_sys::Uint8Array::new(&buf);
            Ok(WsMessage::Binary(array.to_vec()))
        }
        blob if blob.is_instance_of::<web_sys::Blob>() => {
            console_error!("received a blob on the websocket - ignoring it!");
            // we really don't want to bother dealing with Blobs, because it requires juggling filereaders,
            // having event handlers to see when they're done, etc.
            // + we shouldn't even get one [a blob] to begin with
            // considering that our binary mode is (should be) set to array buffer.
            Err(WsError::Io(io::Error::from(io::ErrorKind::InvalidInput)))
        }
        text if text.is_string() => match text.as_string() {
            Some(text) => Ok(WsMessage::Text(text)),
            None => Err(WsError::Utf8),
        },
        _ => Err(WsError::Protocol(Cow::from(
            "received a websocket message that is neither a String, ArrayBuffer or a Blob",
        ))),
    }
}

impl WsStream {
    pub fn new(url: &str) -> Result<Self, JsValue> {
        let ws = WebSocket::new(url)?;
        // we don't want to ever have to deal with blobs
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let message_queue = Rc::new(RefCell::new(VecDeque::new()));
        let message_queue_clone = Rc::clone(&message_queue);

        let stream_waker: Rc<RefCell<Option<Waker>>> = Rc::new(RefCell::new(None));
        let stream_waker_clone = Rc::clone(&stream_waker);

        let sink_waker: Rc<RefCell<Option<Waker>>> = Rc::new(RefCell::new(None));
        let sink_waker_clone = Rc::clone(&sink_waker);

        let on_message = Closure::wrap(Box::new(move |msg_event| {
            let ws_message = match try_message_event_into_ws_message(msg_event) {
                Ok(ws_message) => ws_message,
                Err(err) => {
                    console_error!("failed to read socket message - {}", err);
                    return;
                }
            };
            message_queue_clone.borrow_mut().push_back(ws_message);

            // if there is a task waiting for messages - wake the executor!
            if let Some(waker) = stream_waker_clone.borrow_mut().take() {
                waker.wake()
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        let on_open = Closure::wrap(Box::new(move |foo| {
            console_log!("received a foo {:?}", foo);
            // in case there was a sink send request made before connection was fully established
            console_log!("socket is now open!");

            // if there is a task waiting to write messages - wake the executor!
            if let Some(waker) = sink_waker_clone.borrow_mut().take() {
                waker.wake()
            }

            // no need to wake the stream_waker because we won't have any message to send
            // immediately anyway. It only makes sense to wake it during on_message (if any)
        }) as Box<dyn FnMut(JsValue)>);

        let on_error = Closure::wrap(Box::new(move |e: ErrorEvent| {
            console_error!("error event: {:?}", e);
        }) as Box<dyn FnMut(ErrorEvent)>);

        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));

        Ok(WsStream {
            socket: ws,
            message_queue,
            stream_waker,
            sink_waker,

            _on_open: on_open,
            _on_error: on_error,
            _on_message: on_message,
        })
    }

    fn state(&self) -> State {
        self.socket.ready_state().into()
    }
}

impl Drop for WsStream {
    fn drop(&mut self) {
        match self.state() {
            State::Closed | State::Closing => {} // no need to do anything here
            _ => self
                .socket
                .close()
                .expect("failed to close WebSocket during drop!"), // TODO: how to pass close codes here?
        }
        unimplemented!()
    }
}

impl Stream for WsStream {
    type Item = WsMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // if there's anything in the internal queue, keep returning that
        let ws_message = self.message_queue.borrow_mut().pop_front();
        match ws_message {
            Some(message) => Poll::Ready(Some(message)),
            None => {
                // if connection is closed or closing it means no more useful messages will ever arrive
                // and hence we should signal this.
                match self.state() {
                    State::Closing | State::Closed => Poll::Ready(None),
                    State::Open | State::Connecting => {
                        // clone the waker to be able to notify the executor once we get a new message
                        *self.stream_waker.borrow_mut() = Some(cx.waker().clone());
                        Poll::Pending
                    }
                }
            }
        }
    }
}

impl Sink<WsMessage> for WsStream {
    type Error = WsError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.state() {
            State::Connecting => {
                // clone the waker to be able to notify the executor once we get connected
                *self.sink_waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }

            State::Open => Poll::Ready(Ok(())),
            State::Closing | State::Closed => Poll::Ready(Err(WsError::AlreadyClosed)),
        }
    }

    fn start_send(self: Pin<&mut Self>, item: WsMessage) -> Result<(), Self::Error> {
        // the only possible errors, per https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/send

        // are `INVALID_STATE_ERR` which is when connection is not in open state

        // and `SYNTAX_ERR` which is when data is a string that has unpaired surrogates. This one
        // is essentially impossible to happen in rust (assuming wasm_bindgen has done its jobs
        // correctly, but even if not, there's nothing we can do ourselves.

        // hence we can map all errors to not open

        match self.state() {
            State::Open => match item {
                WsMessage::Binary(data) => self.socket.send_with_u8_array(&data),
                WsMessage::Text(text) => self.socket.send_with_str(&text),
                _ => unreachable!("those are not even exposed by the web_sys API"),
            }
            .map_err(|_| WsError::Io(io::Error::from(io::ErrorKind::NotConnected))),

            State::Closing | State::Closed => Err(WsError::AlreadyClosed),
            State::Connecting => Err(WsError::Io(io::Error::from(io::ErrorKind::NotConnected))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // TODO: can we/should we do anything more here?
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        unimplemented!()
    }
}

// ping, ping and close messages are not explicitly exposed and are handled on the browser-side

// TODO: THIS IS AN EXAMPLE I'VE COPIED BEFORE TO STUDY
//
// #[wasm_bindgen(start)]
// pub fn start_websocket() -> Result<(), JsValue> {
//     // Connect to an echo server
//     let ws = WebSocket::new("wss://echo.websocket.org")?;
//     // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
//     ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
//     // create callback
//     let cloned_ws = ws.clone();
//     let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
//         // Handle difference Text/Binary,...
//         if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
//             console_log!("message event, received arraybuffer: {:?}", abuf);
//             let array = js_sys::Uint8Array::new(&abuf);
//             let len = array.byte_length() as usize;
//             console_log!("Arraybuffer received {}bytes: {:?}", len, array.to_vec());
//             // here you can for example use Serde Deserialize decode the message
//             // for demo purposes we switch back to Blob-type and send off another binary message
//             cloned_ws.set_binary_type(web_sys::BinaryType::Blob);
//             match cloned_ws.send_with_u8_array(&vec![5, 6, 7, 8]) {
//                 Ok(_) => console_log!("binary message successfully sent"),
//                 Err(err) => console_log!("error sending message: {:?}", err),
//             }
//         } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
//             console_log!("message event, received blob: {:?}", blob);
//             // better alternative to juggling with FileReader is to use https://crates.io/crates/gloo-file
//             let fr = web_sys::FileReader::new().unwrap();
//             let fr_c = fr.clone();
//             // create onLoadEnd callback
//             let onloadend_cb = Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
//                 let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
//                 let len = array.byte_length() as usize;
//                 console_log!("Blob received {}bytes: {:?}", len, array.to_vec());
//                 // here you can for example use the received image/png data
//             })
//                 as Box<dyn FnMut(web_sys::ProgressEvent)>);
//             fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
//             fr.read_as_array_buffer(&blob).expect("blob not readable");
//             onloadend_cb.forget();
//         } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
//             console_log!("message event, received Text: {:?}", txt);
//         } else {
//             console_log!("message event, received Unknown: {:?}", e.data());
//         }
//     }) as Box<dyn FnMut(MessageEvent)>);
//     // set message event handler on WebSocket
//     ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
//     // forget the callback to keep it alive
//     onmessage_callback.forget();
//
//     let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
//         console_log!("error event: {:?}", e);
//     }) as Box<dyn FnMut(ErrorEvent)>);
//     ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
//     onerror_callback.forget();
//
//     let cloned_ws = ws.clone();
//     let onopen_callback = Closure::wrap(Box::new(move |_| {
//         console_log!("socket opened");
//         match cloned_ws.send_with_str("ping") {
//             Ok(_) => console_log!("message successfully sent"),
//             Err(err) => console_log!("error sending message: {:?}", err),
//         }
//         // send off binary message
//         match cloned_ws.send_with_u8_array(&vec![0, 1, 2, 3]) {
//             Ok(_) => console_log!("binary message successfully sent"),
//             Err(err) => console_log!("error sending message: {:?}", err),
//         }
//     }) as Box<dyn FnMut(JsValue)>);
//     ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
//     onopen_callback.forget();
//
//     Ok(())
// }
