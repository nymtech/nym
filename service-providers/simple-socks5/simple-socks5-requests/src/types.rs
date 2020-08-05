use std::convert::TryFrom;
use std::fmt::{self, Error, Formatter};

pub type RequestId = u64;

#[derive(Debug)]
pub enum SomeErrorThatNeedsName {
    NotEnoughDataError,
}

impl fmt::Display for SomeErrorThatNeedsName {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        todo!()
    }
}

impl std::error::Error for SomeErrorThatNeedsName {}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum RequestFlag {
    New = 0,
    Send = 1,
    Close = 2,
}

impl TryFrom<u8> for RequestFlag {
    type Error = SomeErrorThatNeedsName;

    fn try_from(value: u8) -> Result<Self, SomeErrorThatNeedsName> {
        match value {
            _ if value == (RequestFlag::New as u8) => Ok(Self::New),
            _ if value == (RequestFlag::Send as u8) => Ok(Self::Send),
            _ if value == (RequestFlag::Close as u8) => Ok(Self::Close),
            _ => todo!("error"),
        }
    }
}

/*
Request:
    Connect: CONN_FLAG || connection_id || address_length || remote_address_bytes  || request_data_content (vec<u8>)
    Send: SEND_FLAG || connection_id || request_data_content (vec<u8>)
    Close: CLOSE_FLAG || connection_id
*/
pub enum Request {
    New(RequestId),
    Send(RequestId, Vec<u8>),
    Close(RequestId),
}

impl Request {
    pub fn try_from_bytes(b: &[u8]) -> Result<Self, SomeErrorThatNeedsName> {
        // each request needs to at least contain flag and RequestId
        if b.len() < 9 {
            return Err(SomeErrorThatNeedsName::NotEnoughDataError);
        }

        // let connection_id = u64::from_be_bytes(b[1..9]); // TODO: probably compiler will complain this is slice not array, but it's fine for time being

        // match RequestFlag::try_from(b[0])? {
        //     RequestFlag::New => todo!(),
        //     RequestFlag::Send => todo!(),
        //     RequestFlag::Close => Request::Close(connection_id),
        // }

        // let total_length = request_bytes.len();
        // let address_length: usize =
        //     (((request_bytes[0] as u16) << 8) | request_bytes[1] as u16).into(); // combines first 2 bytes into one u16
        // let address_start = 2;
        // let address_end = address_start + address_length;
        // let address_vec = request_bytes[address_start..address_end].to_vec();
        // let address = String::from_utf8_lossy(&address_vec).to_string();

        // let request_id_start = address_end;
        // let request_id_end = request_id_start + 16;
        // let request_id_vec = request_bytes[request_id_start..request_id_end].to_vec();
        // let connection_id = Connection::from_slice(&request_id_vec);

        // let data_start = request_id_end;
        // let mut data = Vec::new();
        // if data_start <= total_length {
        //     data = request_bytes[data_start..].to_vec();
        // }
        // (connection_id, address, data)

        todo!()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        todo!()
    }
}

pub enum Response {}

impl Response {
    pub fn try_from_bytes(b: &[u8]) -> Result<Self, SomeErrorThatNeedsName> {
        todo!()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        todo!()
    }
}
