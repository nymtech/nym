// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use mixnet_contract_common::Delegation;

pub(crate) async fn get_single_mixnode_delegations(pubkey: &str) -> Vec<Delegation> {
    let client = crate::client::new_nymd_client();
    let delegates = match client
        .get_all_nymd_single_mixnode_delegations(pubkey.to_string())
        .await
    {
        Ok(result) => result,
        Err(e) => {
            error!("Could not get delegations for mix node {}: {:?}", pubkey, e);
            vec![]
        }
    };
    delegates
}
