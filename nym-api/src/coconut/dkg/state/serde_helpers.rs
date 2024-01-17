// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub(super) mod bte_pk_serde {
    use nym_dkg::bte::PublicKeyWithProof;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        val: &PublicKeyWithProof,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        val.to_bytes().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<PublicKeyWithProof, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<u8> = Deserialize::deserialize(deserializer)?;
        PublicKeyWithProof::try_from_bytes(&vec)
            .map_err(|err| Error::custom(format_args!("{:?}", err)))
    }
}

pub(super) mod vks_serde {
    use nym_dkg::RecoveredVerificationKeys;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        val: &[RecoveredVerificationKeys],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let vec: Vec<Vec<u8>> = val.iter().map(|vk| vk.to_bytes()).collect();
        vec.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<RecoveredVerificationKeys>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<Vec<u8>> = Deserialize::deserialize(deserializer)?;
        vec.into_iter()
            .map(|b| {
                RecoveredVerificationKeys::try_from_bytes(&b)
                    .map_err(|err| D::Error::custom(format_args!("{:?}", err)))
            })
            .collect()
    }
}

pub(super) mod generated_dealings {
    use nym_coconut_dkg_common::types::{DealingIndex, EpochId};
    use nym_dkg::Dealing;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    type Helper = HashMap<EpochId, HashMap<DealingIndex, Vec<u8>>>;

    pub fn serialize<S: Serializer>(
        dealings: &HashMap<EpochId, HashMap<DealingIndex, Dealing>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut helper = HashMap::new();
        for (epoch, dealings) in dealings {
            let mut inner = HashMap::new();
            for (dealing_index, dealing) in dealings {
                inner.insert(*dealing_index, dealing.to_bytes());
            }
            helper.insert(*epoch, inner);
        }
        helper.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HashMap<EpochId, HashMap<DealingIndex, Dealing>>, D::Error> {
        let helper = <Helper>::deserialize(deserializer)?;

        let mut epoch_dealings = HashMap::with_capacity(helper.len());
        for (epoch, dealings) in helper {
            let mut inner = HashMap::with_capacity(dealings.len());
            for (dealing_index, raw_dealing) in dealings {
                let dealing =
                    Dealing::try_from_bytes(&raw_dealing).map_err(serde::de::Error::custom)?;
                inner.insert(dealing_index, dealing);
            }
            epoch_dealings.insert(epoch, inner);
        }
        Ok(epoch_dealings)
    }
}
