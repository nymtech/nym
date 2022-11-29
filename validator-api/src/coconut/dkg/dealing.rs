// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::state::{ConsistentState, State};
use crate::coconut::error::CoconutError;
use coconut_dkg_common::types::TOTAL_DEALINGS;
use contracts_common::dealings::ContractSafeBytes;
use dkg::bte::setup;
use dkg::Dealing;
use rand::RngCore;

pub(crate) async fn dealing_exchange(
    dkg_client: &DkgClient,
    state: &mut State,
    rng: impl RngCore + Clone,
) -> Result<(), CoconutError> {
    if state.receiver_index().is_some() {
        return Ok(());
    }

    let dealers = dkg_client.get_current_dealers().await?;
    // note: ceiling in integer division can be achieved via q = (x + y - 1) / y;
    let threshold = (2 * dealers.len() as u64 + 3 - 1) / 3;

    state.set_dealers(dealers);
    state.set_threshold(threshold);
    let receivers = state.current_dealers_by_idx();
    let params = setup();
    let dealer_index = state.node_index_value()?;
    let receiver_index = receivers
        .keys()
        .position(|node_index| *node_index == dealer_index);
    for _ in 0..TOTAL_DEALINGS {
        let (dealing, _) = Dealing::create(
            rng.clone(),
            &params,
            dealer_index,
            threshold,
            &receivers,
            None,
        );
        dkg_client
            .submit_dealing(ContractSafeBytes::from(&dealing))
            .await?;
    }

    info!("DKG: Finished submitting dealing");
    state.set_receiver_index(receiver_index);

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::coconut::tests::DummyClient;
    use crate::coconut::KeyPair;
    use coconut_dkg_common::dealer::DealerDetails;
    use cosmwasm_std::Addr;
    use dkg::bte::keys::KeyPair as DkgKeyPair;
    use dkg::bte::Params;
    use rand::rngs::OsRng;
    use std::collections::HashMap;
    use std::str::FromStr;
    use std::sync::{Arc, RwLock};
    use url::Url;
    use validator_client::nymd::AccountId;

    const TEST_VALIDATORS_ADDRESS: [&str; 3] = [
        "n1aq9kakfgwqcufr23lsv644apavcntrsqsk4yus",
        "n1s9l3xr4g0rglvk4yctktmck3h4eq0gp6z2e20v",
        "n19kl4py32vsk297dm93ezem992cdyzdy4zuc2x6",
    ];

    fn insert_dealers(
        params: &Params,
        dealer_details_db: &Arc<RwLock<HashMap<String, DealerDetails>>>,
    ) -> Vec<DkgKeyPair> {
        let mut keypairs = vec![];
        for (idx, addr) in TEST_VALIDATORS_ADDRESS.iter().enumerate() {
            let keypair = DkgKeyPair::new(params, OsRng);
            let bte_public_key_with_proof =
                bs58::encode(&keypair.public_key().to_bytes()).into_string();
            keypairs.push(keypair);
            dealer_details_db.write().unwrap().insert(
                addr.to_string(),
                DealerDetails {
                    address: Addr::unchecked(*addr),
                    bte_public_key_with_proof,
                    announce_address: format!("localhost:80{}", idx),
                    assigned_index: (idx + 1) as u64,
                },
            );
        }
        keypairs
    }

    #[tokio::test]
    async fn exchange_dealing() {
        let self_index = 2;
        let dealer_details_db = Arc::new(RwLock::new(HashMap::new()));
        let dealings_db = Arc::new(RwLock::new(HashMap::new()));
        let dkg_client = DkgClient::new(
            DummyClient::new(AccountId::from_str(TEST_VALIDATORS_ADDRESS[0]).unwrap())
                .with_dealer_details(&dealer_details_db)
                .with_dealings(&dealings_db),
        );
        let params = setup();
        let mut state = State::new(
            Url::parse("localhost:8000").unwrap(),
            DkgKeyPair::new(&params, OsRng),
            KeyPair::new(),
        );
        state.set_node_index(Some(self_index));
        let keypairs = insert_dealers(&params, &dealer_details_db);

        dealing_exchange(&dkg_client, &mut state, OsRng)
            .await
            .unwrap();

        assert_eq!(
            state.current_dealers_by_idx().values().collect::<Vec<_>>(),
            keypairs
                .iter()
                .map(|k| k.public_key().public_key())
                .collect::<Vec<_>>()
        );
        assert_eq!(state.threshold().unwrap(), 2);
        assert_eq!(state.receiver_index().unwrap(), 1);
        let dealings = dealings_db
            .read()
            .unwrap()
            .get(TEST_VALIDATORS_ADDRESS[0])
            .unwrap()
            .clone();
        assert_eq!(dealings.len(), TOTAL_DEALINGS);

        dealing_exchange(&dkg_client, &mut state, OsRng)
            .await
            .unwrap();
        let new_dealings = dealings_db
            .read()
            .unwrap()
            .get(TEST_VALIDATORS_ADDRESS[0])
            .unwrap()
            .clone();
        assert_eq!(dealings, new_dealings);
    }
}
