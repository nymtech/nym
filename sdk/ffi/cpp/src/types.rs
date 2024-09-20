// TODO REMOVE when you're working on new CPP branch
#![allow(clippy::all)]
pub mod types {

    use std::ffi::c_char;

    // TODO change all the numbers / replace -2 with prxy?
    #[derive(Debug)]
    pub enum StatusCode {
        NoError = 0,
        ClientInitError = -1,
        // ClientUninitialisedError = -2,
        SelfAddrError = -3,
        SendMsgError = -4,
        ReplyError = -5,
        ListenError = -6,
        RecipientNullError = -7,
        MessageNullError = -8,
    }

    #[repr(C)]
    pub struct CStringCallback {
        pub callback: extern "C" fn(*const c_char),
    }

    impl CStringCallback {
        pub fn new(callback: extern "C" fn(*const c_char)) -> Self {
            CStringCallback { callback }
        }
        pub fn trigger(&self, char: *const c_char) {
            (self.callback)(char);
        }
    }

    #[repr(C)]
    pub struct CMessageCallback {
        pub callback: extern "C" fn(ReceivedMessage),
    }

    impl CMessageCallback {
        pub fn new(callback: extern "C" fn(ReceivedMessage)) -> Self {
            CMessageCallback { callback }
        }
        pub fn trigger(&self, message: ReceivedMessage) {
            (self.callback)(message)
        }
    }

    #[repr(C)]
    pub struct ReceivedMessage {
        pub message: *const u8,
        pub size: usize,
        pub sender_tag: *const c_char,
    }
}
