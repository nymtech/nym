use std::fmt::{Debug, Display};
use std::str::FromStr;

use crate::utilities::hash::{EphemeraHash, EphemeraHasher};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) type EphemeraId = UuidEphemeraIdentifier;

pub(crate) trait EphemeraIdentifier: ToString {
    type Identifier;

    fn generate() -> Self;

    fn inner(&self) -> &Self::Identifier;

    fn into_inner(self) -> Self::Identifier;

    fn as_bytes(&self) -> &[u8];
}

#[derive(Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Deserialize, Serialize)]
pub struct UuidEphemeraIdentifier {
    identifier: String,
}

impl FromStr for UuidEphemeraIdentifier {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UuidEphemeraIdentifier {
            identifier: Uuid::parse_str(s)?.to_string(),
        })
    }
}

impl Display for UuidEphemeraIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identifier)
    }
}

impl Debug for UuidEphemeraIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identifier)
    }
}

impl Default for UuidEphemeraIdentifier {
    fn default() -> Self {
        UuidEphemeraIdentifier {
            identifier: Uuid::new_v4().to_string(),
        }
    }
}

impl EphemeraHash for UuidEphemeraIdentifier {
    fn hash<H: EphemeraHasher>(&self, state: &mut H) -> anyhow::Result<()> {
        state.update(self.as_bytes());
        Ok(())
    }
}

impl EphemeraIdentifier for UuidEphemeraIdentifier {
    type Identifier = String;

    fn generate() -> Self {
        UuidEphemeraIdentifier::default()
    }

    fn inner(&self) -> &Self::Identifier {
        &self.identifier
    }

    fn into_inner(self) -> Self::Identifier {
        self.identifier
    }

    fn as_bytes(&self) -> &[u8] {
        self.identifier.as_bytes()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_generate_parse() {
        let id = UuidEphemeraIdentifier::generate();
        let id2 = UuidEphemeraIdentifier::from_str(&id.to_string()).unwrap();
        assert_eq!(id, id2);
    }
}
