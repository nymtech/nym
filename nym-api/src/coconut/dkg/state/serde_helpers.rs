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

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Box<PublicKeyWithProof>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<u8> = Deserialize::deserialize(deserializer)?;
        PublicKeyWithProof::try_from_bytes(&vec)
            .map_err(|err| Error::custom(format_args!("{:?}", err)))
            .map(Box::new)
    }
}

pub(super) mod recovered_keys {
    use nym_coconut_dkg_common::types::DealingIndex;
    use nym_dkg::RecoveredVerificationKeys;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    type Helper = BTreeMap<DealingIndex, Vec<u8>>;

    pub fn serialize<S: Serializer>(
        val: &BTreeMap<DealingIndex, RecoveredVerificationKeys>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let helper: Helper = val
            .iter()
            .map(|(idx, rec)| (*idx, rec.to_bytes()))
            .collect();
        helper.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<BTreeMap<DealingIndex, RecoveredVerificationKeys>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = Helper::deserialize(deserializer)?;
        helper
            .into_iter()
            .map(|(idx, rec)| {
                RecoveredVerificationKeys::try_from_bytes(&rec)
                    .map_err(|err| D::Error::custom(format_args!("{:?}", err)))
                    .map(|vk| (idx, vk))
            })
            .collect()
    }
}

pub(super) mod generated_dealings {
    use nym_coconut_dkg_common::types::DealingIndex;
    use nym_dkg::Dealing;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S: Serializer>(
        dealings: &HashMap<DealingIndex, Dealing>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut helper = HashMap::new();

        for (dealing_index, dealing) in dealings {
            helper.insert(*dealing_index, dealing.to_bytes());
        }

        helper.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HashMap<DealingIndex, Dealing>, D::Error> {
        <HashMap<DealingIndex, Vec<u8>>>::deserialize(deserializer)?
            .into_iter()
            .map(|(index, raw_dealing)| {
                Dealing::try_from_bytes(&raw_dealing)
                    .map_err(serde::de::Error::custom)
                    .map(|dealing| (index, dealing))
            })
            .collect()
    }
}
