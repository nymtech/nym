// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::websocket::connection_handler::fresh::InitialAuthenticationError;
use nym_gateway_requests::SharedGatewayKey;
use nym_gateway_storage::models::PersistedSharedKeys;
use nym_sphinx::DestinationAddressBytes;
use time::OffsetDateTime;

pub(crate) struct KeyWithAuthTimestamp {
    pub(crate) client_id: i64,
    pub(crate) key: SharedGatewayKey,
    pub(crate) last_used_authentication: Option<OffsetDateTime>,
}

impl KeyWithAuthTimestamp {
    pub(crate) fn try_from_stored(
        stored_shared_keys: PersistedSharedKeys,
        client: DestinationAddressBytes,
    ) -> Result<Self, InitialAuthenticationError> {
        let last_used_authentication = stored_shared_keys.last_used_authentication;
        let client_id = stored_shared_keys.client_id;

        let key = SharedGatewayKey::try_from(stored_shared_keys).map_err(|source| {
            InitialAuthenticationError::MalformedStoredSharedKey {
                client_id: client.as_base58_string(),
                source,
            }
        })?;

        Ok(KeyWithAuthTimestamp {
            client_id,
            key,
            last_used_authentication,
        })
    }
}
