// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::nyxd;
use nym_credential_storage::persistent_storage::PersistentStorage;
use nym_crypto::asymmetric::identity::PUBLIC_KEY_LENGTH;
use nym_gateway_client::GatewayClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard, TryLockError};

pub(crate) struct GatewayClientHandle(Arc<GatewayClientHandleInner>);

struct GatewayClientHandleInner {
    client: Mutex<Option<GatewayClient<nyxd::Client, PersistentStorage>>>,
    raw_identity: [u8; PUBLIC_KEY_LENGTH],
}

pub(crate) struct UnlockedGatewayClientHandle<'a>(
    MutexGuard<'a, Option<GatewayClient<nyxd::Client, PersistentStorage>>>,
);

impl GatewayClientHandle {
    pub(crate) fn new(gateway_client: GatewayClient<nyxd::Client, PersistentStorage>) -> Self {
        GatewayClientHandle(Arc::new(GatewayClientHandleInner {
            raw_identity: gateway_client.gateway_identity().to_bytes(),
            client: Mutex::new(Some(gateway_client)),
        }))
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    // this could have also been achieved with a normal #[derive(Clone)] but I prefer to be explicit about it,
    // because clippy would suggest some potentially confusing 'simplifications' regarding clone
    pub(crate) fn clone_data_pointer(&self) -> Self {
        GatewayClientHandle(Arc::clone(&self.0))
    }

    pub(crate) fn raw_identity(&self) -> [u8; PUBLIC_KEY_LENGTH] {
        self.0.raw_identity
    }

    pub(crate) async fn is_invalid(&self) -> bool {
        self.0.client.lock().await.is_none()
    }

    pub(crate) async fn lock_client(&self) -> UnlockedGatewayClientHandle<'_> {
        UnlockedGatewayClientHandle(self.0.client.lock().await)
    }

    pub(crate) fn lock_client_unchecked(&self) -> UnlockedGatewayClientHandle<'_> {
        UnlockedGatewayClientHandle(self.0.client.try_lock().unwrap())
    }

    pub(crate) fn try_lock_client(&self) -> Result<UnlockedGatewayClientHandle<'_>, TryLockError> {
        self.0.client.try_lock().map(UnlockedGatewayClientHandle)
    }
}

impl<'a> UnlockedGatewayClientHandle<'a> {
    pub(crate) fn get_mut_unchecked(
        &mut self,
    ) -> &mut GatewayClient<nyxd::Client, PersistentStorage> {
        self.0.as_mut().unwrap()
    }

    pub(crate) fn inner_mut(
        &mut self,
    ) -> Option<&mut GatewayClient<nyxd::Client, PersistentStorage>> {
        self.0.as_mut()
    }

    pub(crate) fn invalidate(&mut self) {
        *self.0 = None
    }
}

pub(crate) type GatewayClientsMap = HashMap<[u8; PUBLIC_KEY_LENGTH], GatewayClientHandle>;

#[derive(Clone)]
pub(crate) struct ActiveGatewayClients {
    // there is no point in using an RwLock here as there will only ever be two readers here and both
    // potentially need write access.
    // A BiLock would have been slightly better than a normal Mutex since it's optimised for two
    // owners, but it's behind `unstable` feature flag in futures and it would be a headache if the API
    // changed.
    inner: Arc<Mutex<GatewayClientsMap>>,
}

impl ActiveGatewayClients {
    pub(crate) fn new() -> Self {
        ActiveGatewayClients {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) async fn lock(&self) -> MutexGuard<'_, GatewayClientsMap> {
        self.inner.lock().await
    }
}
