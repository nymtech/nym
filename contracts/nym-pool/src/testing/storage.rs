// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::storage_keys::to_length_prefixed_nested;
use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{Addr, MemoryStorage, Order, Record, Storage};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct StorageWrapper(Rc<RefCell<MemoryStorage>>);

impl StorageWrapper {
    pub(super) fn contract_storage_wrapper(&self, contract: &Addr) -> ContractStorageWrapper {
        ContractStorageWrapper {
            address: contract.clone(),
            inner: self.clone(),
        }
    }

    pub(super) fn new() -> Self {
        StorageWrapper(Rc::new(RefCell::new(MockStorage::new())))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ContractStorageWrapper {
    address: Addr,
    inner: StorageWrapper,
}

impl ContractStorageWrapper {
    pub fn inner_storage(&self) -> StorageWrapper {
        self.inner.clone()
    }
}

impl Storage for StorageWrapper {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.borrow().get(key)
    }

    fn range<'a>(
        &'a self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        // hehe, that's nasty
        let vals = self.0.borrow().range(start, end, order).collect::<Vec<_>>();
        Box::new(vals.into_iter())
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.0.borrow_mut().set(key, value);
    }

    fn remove(&mut self, key: &[u8]) {
        self.0.borrow_mut().remove(key);
    }
}

impl ContractStorageWrapper {
    fn contract_namespace(&self) -> Vec<u8> {
        let mut name = b"contract_data/".to_vec();
        name.extend_from_slice(self.address.as_bytes());
        name
    }

    fn prefix(&self) -> Vec<u8> {
        to_length_prefixed_nested(&[b"wasm", &self.contract_namespace()])
    }

    #[inline]
    fn trim(namespace: &[u8], key: &[u8]) -> Vec<u8> {
        key[namespace.len()..].to_vec()
    }

    /// Returns a new vec of same length and last byte incremented by one
    /// If last bytes are 255, we handle overflow up the chain.
    /// If all bytes are 255, this returns wrong data - but that is never possible as a namespace
    fn namespace_upper_bound(input: &[u8]) -> Vec<u8> {
        let mut copy = input.to_vec();
        // zero out all trailing 255, increment first that is not such
        for i in (0..input.len()).rev() {
            if copy[i] == 255 {
                copy[i] = 0;
            } else {
                copy[i] += 1;
                break;
            }
        }
        copy
    }

    #[inline]
    fn concat(namespace: &[u8], key: &[u8]) -> Vec<u8> {
        let mut k = namespace.to_vec();
        k.extend_from_slice(key);
        k
    }

    fn get_with_prefix(storage: &dyn Storage, namespace: &[u8], key: &[u8]) -> Option<Vec<u8>> {
        storage.get(&Self::concat(namespace, key))
    }

    fn set_with_prefix(storage: &mut dyn Storage, namespace: &[u8], key: &[u8], value: &[u8]) {
        storage.set(&Self::concat(namespace, key), value);
    }

    fn remove_with_prefix(storage: &mut dyn Storage, namespace: &[u8], key: &[u8]) {
        storage.remove(&Self::concat(namespace, key));
    }

    fn range_with_prefix<'a>(
        storage: &'a dyn Storage,
        namespace: &[u8],
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        // prepare start, end with prefix
        let start = match start {
            Some(s) => Self::concat(namespace, s),
            None => namespace.to_vec(),
        };
        let end = match end {
            Some(e) => Self::concat(namespace, e),
            // end is updating last byte by one
            None => Self::namespace_upper_bound(namespace),
        };

        // get iterator from storage
        let base_iterator = storage.range(Some(&start), Some(&end), order);

        // make a copy for the closure to handle lifetimes safely
        let prefix = namespace.to_vec();
        let mapped = base_iterator.map(move |(k, v)| (Self::trim(&prefix, &k), v));
        Box::new(mapped)
    }
}

impl Storage for ContractStorageWrapper {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let prefix = self.prefix();
        Self::get_with_prefix(&self.inner, &prefix, key)
    }

    fn range<'a>(
        &'a self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        let prefix = self.prefix();
        Self::range_with_prefix(&self.inner, &prefix, start, end, order)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        let prefix = self.prefix();
        Self::set_with_prefix(&mut self.inner, &prefix, key, value);
    }

    fn remove(&mut self, key: &[u8]) {
        let prefix = self.prefix();
        Self::remove_with_prefix(&mut self.inner, &prefix, key);
    }
}
