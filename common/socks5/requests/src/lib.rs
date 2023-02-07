// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use service_providers_common::interface;
use service_providers_common::interface::ServiceProviderMessagingError;
use thiserror::Error;

pub use request::*;
pub use response::*;
pub use version::*;

pub mod request;
pub mod response;
pub mod version;

pub type Socks5ProviderRequest = interface::Request<Socks5Request>;
pub type Socks5ProviderResponse = interface::Response<Socks5Request>;

#[derive(Debug, Error)]
pub enum Socks5RequestError {
    #[error("failed to deserialize received request: {source}")]
    RequestDeserialization {
        #[from]
        source: RequestDeserializationError,
    },

    #[error("failed to deserialize received response: {source}")]
    ResponseDeserialization {
        #[from]
        source: ResponseDeserializationError,
    },

    #[error(transparent)]
    ProviderInterfaceError(#[from] ServiceProviderMessagingError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use service_providers_common::interface::RequestContent;

    #[cfg(test)]
    mod interface_backwards_compatibility {
        use super::*;
        use service_providers_common::interface::ProviderInterfaceVersion;

        #[test]
        fn old_client_vs_new_service_provider() {
            let old_serialized_connect = vec![
                0, 0, 2, 254, 34, 100, 192, 20, 13, 171, 0, 16, 56, 48, 46, 50, 52, 57, 46, 57, 57,
                46, 49, 52, 56, 58, 56, 48, 34, 112, 17, 182, 225, 6, 174, 216, 160, 41, 72, 236,
                160, 90, 156, 3, 250, 41, 243, 53, 191, 178, 218, 53, 170, 14, 185, 33, 94, 153,
                25, 41, 6, 82, 169, 187, 88, 246, 211, 57, 68, 225, 228, 231, 116, 29, 119, 235,
                160, 14, 156, 205, 66, 1, 75, 204, 204, 220, 14, 150, 191, 203, 174, 88, 121, 173,
                83, 219, 188, 164, 194, 212, 238, 228, 4, 128, 48, 105, 224, 83, 17, 246, 233, 16,
                235, 223, 68, 87, 13, 40, 34, 186, 218, 204, 126, 145,
            ];

            let new_deserialized =
                Socks5ProviderRequest::try_from_bytes(&old_serialized_connect).unwrap();

            match new_deserialized.content {
                RequestContent::ProviderData(req) => match req.content {
                    Socks5RequestContent::Connect(connect_req) => {
                        assert_eq!(connect_req.remote_addr, "80.249.99.148:80".to_string());
                        assert_eq!(connect_req.conn_id, 215647648274976171);
                        assert_eq!(connect_req.return_address, Some("3KRydEpanwjFhq5GAraVjRUF1Tno7w7oc4EwJYTGNo5J.RgZ7uMJHruBQqD5hC9Ghi3sqiTn6NycfM5qCfJz6yoM@9Byd9VAtyYMnbVAcqdoQxJnq76XEg2dbxbiF5Aa5Jj9J".parse().unwrap()));
                    }
                    _ => panic!("unexpected request"),
                },
                _ => panic!("unexpected request"),
            }

            let old_serialized_send = vec![
                0, 1, 108, 102, 28, 19, 50, 178, 37, 241, 0, 0, 0, 0, 0, 0, 0, 0, 0, 71, 69, 84,
                32, 47, 49, 77, 66, 46, 122, 105, 112, 32, 72, 84, 84, 80, 47, 49, 46, 49, 13, 10,
                72, 111, 115, 116, 58, 32, 105, 112, 118, 52, 46, 100, 111, 119, 110, 108, 111, 97,
                100, 46, 116, 104, 105, 110, 107, 98, 114, 111, 97, 100, 98, 97, 110, 100, 46, 99,
                111, 109, 13, 10, 85, 115, 101, 114, 45, 65, 103, 101, 110, 116, 58, 32, 99, 117,
                114, 108, 47, 55, 46, 54, 56, 46, 48, 13, 10, 65, 99, 99, 101, 112, 116, 58, 32,
                42, 47, 42, 13, 10, 13, 10,
            ];

            let new_deserialized =
                Socks5ProviderRequest::try_from_bytes(&old_serialized_send).unwrap();

            match new_deserialized.content {
                RequestContent::ProviderData(req) => match req.content {
                    Socks5RequestContent::Send(send_req) => {
                        assert_eq!(send_req.conn_id, 7810961472501196273);
                        assert_eq!(send_req.data.len(), 111);
                        assert!(!send_req.local_closed);
                    }
                    _ => panic!("unexpected request"),
                },
                _ => panic!("unexpected request"),
            }
        }

        #[test]
        fn new_client_vs_old_service_provider() {
            let return_address = "3KRydEpanwjFhq5GAraVjRUF1Tno7w7oc4EwJYTGNo5J.RgZ7uMJHruBQqD5hC9Ghi3sqiTn6NycfM5qCfJz6yoM@9Byd9VAtyYMnbVAcqdoQxJnq76XEg2dbxbiF5Aa5Jj9J".parse().unwrap();

            let new_connect = Socks5ProviderRequest::new_provider_data(
                ProviderInterfaceVersion::Legacy,
                Socks5Request::new_connect(
                    Socks5ProtocolVersion::Legacy,
                    215647648274976171,
                    "80.249.99.148:80".to_string(),
                    Some(return_address),
                ),
            );

            let legacy_serialised = new_connect.into_bytes();
            let old_serialized_connect = vec![
                0, 0, 2, 254, 34, 100, 192, 20, 13, 171, 0, 16, 56, 48, 46, 50, 52, 57, 46, 57, 57,
                46, 49, 52, 56, 58, 56, 48, 34, 112, 17, 182, 225, 6, 174, 216, 160, 41, 72, 236,
                160, 90, 156, 3, 250, 41, 243, 53, 191, 178, 218, 53, 170, 14, 185, 33, 94, 153,
                25, 41, 6, 82, 169, 187, 88, 246, 211, 57, 68, 225, 228, 231, 116, 29, 119, 235,
                160, 14, 156, 205, 66, 1, 75, 204, 204, 220, 14, 150, 191, 203, 174, 88, 121, 173,
                83, 219, 188, 164, 194, 212, 238, 228, 4, 128, 48, 105, 224, 83, 17, 246, 233, 16,
                235, 223, 68, 87, 13, 40, 34, 186, 218, 204, 126, 145,
            ];

            assert_eq!(legacy_serialised, old_serialized_connect);
        }
    }
}
