// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::PublicKey;

pub mod bs58_ed25519_pubkey {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(key: &PublicKey, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&key.to_base58_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PublicKey, D::Error> {
        let s = String::deserialize(deserializer)?;
        PublicKey::from_base58_string(s).map_err(serde::de::Error::custom)
    }
}

pub mod vec_bs58_ed25519_pubkey {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer, ser::SerializeSeq};

    pub fn serialize<S: Serializer>(
        keys: &Vec<PublicKey>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(keys.len()))?;
        for key in keys {
            seq.serialize_element(&Bs58KeyWrapper(*key))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<PublicKey>, D::Error> {
        let wrapped = Vec::<Bs58KeyWrapper>::deserialize(deserializer)?;
        Ok(wrapped.into_iter().map(|k| k.0).collect())
    }

    struct Bs58KeyWrapper(PublicKey);

    impl serde::Serialize for Bs58KeyWrapper {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            bs58_ed25519_pubkey::serialize(&self.0, serializer)
        }
    }

    impl<'de> Deserialize<'de> for Bs58KeyWrapper {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            Ok(Bs58KeyWrapper(bs58_ed25519_pubkey::deserialize(
                deserializer,
            )?))
        }
    }
}

pub mod bs58_ed25519_signature {
    use crate::asymmetric::ed25519::Signature;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        signature: &Signature,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&signature.to_base58_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Signature, D::Error> {
        let s = String::deserialize(deserializer)?;
        Signature::from_base58_string(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jwt_simple::reexports::{anyhow, serde_json};
    use nym_test_utils::helpers::deterministic_rng;
    use serde::{Deserialize, Serialize};

    #[test]
    fn vec_bs58_ed25519_pubkey_json() -> anyhow::Result<()> {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct KeysWrapper(#[serde(with = "vec_bs58_ed25519_pubkey")] Vec<PublicKey>);

        use crate::asymmetric::ed25519;
        let mut rng = deterministic_rng();
        let empty = KeysWrapper(vec![]);
        let single_key = KeysWrapper(vec![PublicKey::from_base58_string(
            "Be9wH7xuXBRJAuV1pC7MALZv6a61RvWQ3SypsNarqTt",
        )?]);
        let three_keys = KeysWrapper(vec![
            ed25519::KeyPair::new(&mut rng).public_key,
            ed25519::KeyPair::new(&mut rng).public_key,
            ed25519::KeyPair::new(&mut rng).public_key,
        ]);

        let se_empty = serde_json::to_string(&empty)?;
        let se_single_key = serde_json::to_string(&single_key)?;
        let se_three_keys = serde_json::to_string(&three_keys)?;

        assert_eq!(se_empty, r#"[]"#);
        assert_eq!(
            se_single_key,
            r#"["Be9wH7xuXBRJAuV1pC7MALZv6a61RvWQ3SypsNarqTt"]"#
        );
        assert_eq!(
            se_three_keys,
            r#"["HmgHDV79LpnEaSUp8QZQwSroxVvS4RewF7yM9e7qu8y3","311xRh859qCd5MVqoPRCoNx26eYhLknGwtjzkkTJFGhf","A5BMp8WJ6Uk91U4JpWRv2Bc6X35AaRaSEy8QEWeAkaBv"]"#
        );

        let empty_de = serde_json::from_str::<KeysWrapper>(&se_empty)?;
        let single_key_de = serde_json::from_str::<KeysWrapper>(&se_single_key)?;
        let three_keys_de = serde_json::from_str::<KeysWrapper>(&se_three_keys)?;

        assert_eq!(empty, empty_de);
        assert_eq!(single_key, single_key_de);
        assert_eq!(three_keys, three_keys_de);

        Ok(())
    }
}
