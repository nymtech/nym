// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod argon2_params_helper {
    use argon2::Params;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Refer to [argon2::Params] for details.
    #[derive(Serialize, Deserialize)]
    struct Argon2Params {
        m_cost: u32,
        t_cost: u32,
        p_cost: u32,
        output_len: Option<usize>,
        // Note: `keyid` and `data` are not longer part of the argon2 standard
        // (see: <https://github.com/P-H-C/phc-winner-argon2/pull/173>), and should
        // not be used for any non-legacy work.
        // So we're explicitly skipping them for serialization
    }

    impl From<&Params> for Argon2Params {
        fn from(value: &Params) -> Self {
            Argon2Params {
                m_cost: value.m_cost(),
                t_cost: value.t_cost(),
                p_cost: value.p_cost(),
                output_len: value.output_len(),
            }
        }
    }

    impl TryFrom<Argon2Params> for Params {
        type Error = argon2::Error;

        fn try_from(value: Argon2Params) -> Result<Self, Self::Error> {
            Params::new(value.m_cost, value.t_cost, value.p_cost, value.output_len)
        }
    }

    pub fn serialize<S: Serializer>(params: &Params, serializer: S) -> Result<S::Ok, S::Error> {
        Argon2Params::from(params).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Params, D::Error> {
        <Argon2Params>::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

pub(crate) mod argon2_algorithm_helper {
    use argon2::Algorithm;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Refer to [argon2::Algorithm] for details.
    #[derive(Serialize, Deserialize)]
    enum Argon2Algorithm {
        Argon2d = 0,
        Argon2i = 1,
        Argon2id = 2,
    }

    impl From<Algorithm> for Argon2Algorithm {
        fn from(value: Algorithm) -> Self {
            match value {
                Algorithm::Argon2d => Argon2Algorithm::Argon2d,
                Algorithm::Argon2i => Argon2Algorithm::Argon2i,
                Algorithm::Argon2id => Argon2Algorithm::Argon2id,
            }
        }
    }

    impl From<Argon2Algorithm> for Algorithm {
        fn from(value: Argon2Algorithm) -> Self {
            match value {
                Argon2Algorithm::Argon2d => Algorithm::Argon2d,
                Argon2Algorithm::Argon2i => Algorithm::Argon2i,
                Argon2Algorithm::Argon2id => Algorithm::Argon2id,
            }
        }
    }

    pub fn serialize<S: Serializer>(
        algorithm: &Algorithm,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        Argon2Algorithm::from(*algorithm).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Algorithm, D::Error> {
        <Argon2Algorithm>::deserialize(deserializer).map(From::from)
    }
}

pub(crate) mod argon2_version_helper {
    use argon2::Version;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Refer to [argon2::Version] for details.
    #[derive(Serialize, Deserialize)]
    #[repr(u32)]
    enum Argon2Version {
        V0x10 = 0x10,
        V0x13 = 0x13,
    }

    impl From<Version> for Argon2Version {
        fn from(value: Version) -> Self {
            match value {
                Version::V0x10 => Argon2Version::V0x10,
                Version::V0x13 => Argon2Version::V0x13,
            }
        }
    }

    impl From<Argon2Version> for Version {
        fn from(value: Argon2Version) -> Self {
            match value {
                Argon2Version::V0x10 => Version::V0x10,
                Argon2Version::V0x13 => Version::V0x13,
            }
        }
    }

    pub fn serialize<S: Serializer>(algorithm: &Version, serializer: S) -> Result<S::Ok, S::Error> {
        Argon2Version::from(*algorithm).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Version, D::Error> {
        <Argon2Version>::deserialize(deserializer).map(From::from)
    }
}
