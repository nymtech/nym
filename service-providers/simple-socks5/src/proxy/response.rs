use crate::proxy::request::RequestId;

pub struct Response {
    data: Vec<u8>,
    request_id: RequestId,
}

impl Response {
    pub fn new(request_id: [u8; 16], data: Vec<u8>) -> Response {
        Response { data, request_id }
    }

    /// Serializes the response as `request_id || data`, returning a byte vector
    pub fn serialize(self) -> Vec<u8> {
        self.request_id
            .iter()
            .cloned()
            .chain(self.data.into_iter())
            .collect()
    }
}
