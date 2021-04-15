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

use crate::websocket::state::State;
use crate::{console_error, console_log};
use futures::{Sink, Stream};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use tungstenite::{Error as WsError, Message as WsMessage}; // use tungstenite Message and Error types for easier compatibility with `ClientHandshake`
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

mod state;

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
        // "received a websocket message that is neither a String, ArrayBuffer or a Blob"
        _ => Err(WsError::Io(io::Error::from(io::ErrorKind::InvalidInput))),
    }
}

// Safety: when compiled to wasm32 everything is going to be running on a single thread and so there
// is no shared memory right now.
//
// Eventually it should be made `Send` properly. Wakers should probably be replaced with AtomicWaker
// and the item queue put behind an Arc<Mutex<...>>.
// It might also be worth looking at what https://crates.io/crates/send_wrapper could provide.
// Because I'm not sure Mutex would solve the `Closure` issue. It's the problem for later.
//
// ************************************
// SUPER IMPORTANT TODO: ONCE WASM IN RUST MATURES AND BECOMES MULTI-THREADED THIS MIGHT
// LEAD TO RUNTIME MEMORY CORRUPTION!!
// ************************************
//
unsafe impl Send for JSWebsocket {}

#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct JSWebsocket {
    socket: web_sys::WebSocket,

    message_queue: Rc<RefCell<VecDeque<Result<WsMessage, WsError>>>>,

    /// Waker of a task wanting to read incoming messages.
    stream_waker: Rc<RefCell<Option<Waker>>>,

    /// Waker of a task wanting to write to the sink.
    sink_waker: Rc<RefCell<Option<Waker>>>,

    /// Waker of a sink wanting to close the connection.
    close_waker: Rc<RefCell<Option<Waker>>>,

    // The callback closures. We need to store them as they will invalidate their
    // corresponding JS callback whenever they are dropped, so if we were to
    // normally return from `new` then our registered closures will
    // raise an exception when invoked.
    _on_open: Closure<dyn FnMut(JsValue)>,
    _on_error: Closure<dyn FnMut(ErrorEvent)>,
    _on_close: Closure<dyn FnMut(CloseEvent)>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl JSWebsocket {
    pub fn new(url: &str) -> Result<Self, JsValue> {
        let ws = WebSocket::new(url)?;
        // we don't want to ever have to deal with blobs
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let message_queue = Rc::new(RefCell::new(VecDeque::new()));
        let message_queue_clone = Rc::clone(&message_queue);

        let stream_waker: Rc<RefCell<Option<Waker>>> = Rc::new(RefCell::new(None));
        let stream_waker_clone = Rc::clone(&stream_waker);
        let stream_waker_clone2 = Rc::clone(&stream_waker);

        let sink_waker: Rc<RefCell<Option<Waker>>> = Rc::new(RefCell::new(None));
        let sink_waker_clone = Rc::clone(&sink_waker);
        let sink_waker_clone2 = Rc::clone(&sink_waker);

        let close_waker: Rc<RefCell<Option<Waker>>> = Rc::new(RefCell::new(None));
        let close_waker_clone = Rc::clone(&close_waker);

        let on_message = Closure::wrap(Box::new(move |msg_event| {
            let ws_message = try_message_event_into_ws_message(msg_event);
            message_queue_clone.borrow_mut().push_back(ws_message);

            // if there is a task waiting for messages - wake the executor!
            if let Some(waker) = stream_waker_clone.borrow_mut().take() {
                waker.wake()
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        let url_clone = url.to_string();
        let on_open = Closure::wrap(Box::new(move |_| {
            // in case there was a sink send request made before connection was fully established
            console_log!("Websocket to {:?} is now open!", url_clone);

            // if there is a task waiting to write messages - wake the executor!
            if let Some(waker) = sink_waker_clone.borrow_mut().take() {
                waker.wake()
            }

            // no need to wake the stream_waker because we won't have any message to send
            // immediately anyway. It only makes sense to wake it during on_message (if any)
        }) as Box<dyn FnMut(JsValue)>);

        let on_error = Closure::wrap(Box::new(move |e: ErrorEvent| {
            console_error!("Websocket error event: {:?}", e);
        }) as Box<dyn FnMut(ErrorEvent)>);

        let on_close = Closure::wrap(Box::new(move |e: CloseEvent| {
            console_log!("Websocket close event: {:?}", e);
            // something was waiting for the close event!
            if let Some(waker) = close_waker_clone.borrow_mut().take() {
                waker.wake()
            }

            // TODO: are waking those sufficient to prevent memory leaks?
            if let Some(waker) = stream_waker_clone2.borrow_mut().take() {
                waker.wake()
            }

            if let Some(waker) = sink_waker_clone2.borrow_mut().take() {
                waker.wake()
            }
        }) as Box<dyn FnMut(CloseEvent)>);

        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

        Ok(JSWebsocket {
            socket: ws,
            message_queue,
            stream_waker,
            sink_waker,
            close_waker,

            _on_open: on_open,
            _on_error: on_error,
            _on_close: on_close,
            _on_message: on_message,
        })
    }

    pub async fn close(&mut self, code: Option<u16>) {
        if let Some(code) = code {
            self.socket
                .close_with_code(code)
                .expect("failed to close the socket!");
        } else {
            self.socket.close().expect("failed to close the socket!");
        }
    }

    fn state(&self) -> State {
        self.socket.ready_state().into()
    }
}

impl Drop for JSWebsocket {
    fn drop(&mut self) {
        match self.state() {
            State::Closed | State::Closing => {} // no need to do anything here
            _ => self
                .socket
                .close()
                .expect("failed to close WebSocket during drop!"),
        }

        self.socket.set_onmessage(None);
        self.socket.set_onerror(None);
        self.socket.set_onopen(None);
        self.socket.set_onclose(None);
    }
}

impl Stream for JSWebsocket {
    type Item = Result<WsMessage, WsError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
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

impl Sink<WsMessage> for JSWebsocket {
    type Error = WsError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
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

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // TODO: can we/should we do anything more here?
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.state() {
            State::Open | State::Connecting => {
                // TODO: do we need to wait for closing event here?
                *self.close_waker.borrow_mut() = Some(cx.waker().clone());

                // close inner socket
                Poll::Ready(self.socket.close().map_err(|_| todo!()))
            }
            // if we're already closed, nothing left to do!
            State::Closed => Poll::Ready(Ok(())),
            State::Closing => {
                *self.close_waker.borrow_mut() = Some(cx.waker().clone());
                // wait for the close event...
                Poll::Pending
            }
        }
    }
}
