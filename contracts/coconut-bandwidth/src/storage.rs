// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::{Index, IndexList, IndexedMap, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub enum SpendCredentialStatus {
    InProgress,
    Spent,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct SpendCredential {
    funds: Coin,
    blinded_serial_number: String,
    gateway_cosmos_address: Addr,
    status: SpendCredentialStatus,
}

impl SpendCredential {
    pub fn new(funds: Coin, blinded_serial_number: String, gateway_cosmos_address: Addr) -> Self {
        SpendCredential {
            funds,
            blinded_serial_number,
            gateway_cosmos_address,
            status: SpendCredentialStatus::InProgress,
        }
    }

    pub fn mark_as_spent(&mut self) {
        self.status = SpendCredentialStatus::Spent;
    }
}

// storage prefixes
const SPEND_CREDENTIAL_PK_NAMESPACE: &str = "sc";
const SPEND_CREDENTIAL_BLINDED_SERIAL_NO_IDX_NAMESPACE: &str = "scn";

pub(crate) struct SpendCredentialIndex<'a> {
    pub(crate) blinded_serial_number: UniqueIndex<'a, String, SpendCredential>,
}

// IndexList is just boilerplate code for fetching a struct's indexes
// note that from my understanding this will be converted into a macro at some point in the future
impl<'a> IndexList<SpendCredential> for SpendCredentialIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<SpendCredential>> + '_> {
        let v: Vec<&dyn Index<SpendCredential>> = vec![&self.blinded_serial_number];
        Box::new(v.into_iter())
    }
}

// gateways() is the storage access function.
pub(crate) fn spent_credentials<'a>(
) -> IndexedMap<'a, &'a str, SpendCredential, SpendCredentialIndex<'a>> {
    let indexes = SpendCredentialIndex {
        blinded_serial_number: UniqueIndex::new(
            |d| d.blinded_serial_number.clone(),
            SPEND_CREDENTIAL_BLINDED_SERIAL_NO_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(SPEND_CREDENTIAL_PK_NAMESPACE, indexes)
}

// currently not used outside tests
#[cfg(test)]
mod tests {
    use super::super::storage;
    use crate::storage::SpendCredential;
    use crate::storage::SpendCredentialStatus;
    use crate::support::tests::fixtures;
    use config::defaults::MIX_DENOM;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::Addr;
    use cosmwasm_std::Coin;

    #[test]
    fn spend_credential_single_read_retrieval() {
        let mut storage = MockStorage::new();
        let blind_serial_number1 = "number1";
        let blind_serial_number2 = "number2";
        let spend1 = fixtures::spend_credential_fixture(blind_serial_number1);
        let spend2 = fixtures::spend_credential_fixture(blind_serial_number2);
        storage::spent_credentials()
            .save(&mut storage, blind_serial_number1, &spend1)
            .unwrap();
        storage::spent_credentials()
            .save(&mut storage, blind_serial_number2, &spend2)
            .unwrap();

        let res1 = storage::spent_credentials()
            .load(&storage, blind_serial_number1)
            .unwrap();
        let res2 = storage::spent_credentials()
            .load(&storage, blind_serial_number2)
            .unwrap();
        assert_eq!(spend1, res1);
        assert_eq!(spend2, res2);
    }

    #[test]
    fn mark_as_spent_credential() {
        let mut mock_storage = MockStorage::new();
        let funds = Coin::new(100, MIX_DENOM.base);
        let blind_serial_number = "blind_serial_number";
        let gateway_cosmos_address: Addr = Addr::unchecked("gateway_cosmos_address");

        let res = storage::spent_credentials()
            .may_load(&mock_storage, blind_serial_number)
            .unwrap();
        assert!(res.is_none());

        let mut spend_credential = SpendCredential::new(
            funds.clone(),
            blind_serial_number.to_string(),
            gateway_cosmos_address.clone(),
        );
        spend_credential.mark_as_spent();

        storage::spent_credentials()
            .save(&mut mock_storage, blind_serial_number, &spend_credential)
            .unwrap();

        let ret = storage::spent_credentials()
            .load(&mock_storage, blind_serial_number)
            .unwrap();

        assert_eq!(ret, spend_credential);
        assert_eq!(ret.status, SpendCredentialStatus::Spent);
    }
}
