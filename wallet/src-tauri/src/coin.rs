// This should be moved out of the wallet, and used as a primary coin type throughout the codebase

use crate::error::BackendError;
use crate::network::Network;
use serde::{Deserialize, Serialize};

use std::str::FromStr;
use strum::IntoEnumIterator;
use nym_validator_client::nyxd::CosmosCoin;
use nym_validator_client::nyxd::Denom as CosmosDenom;
use nym_validator_client::nyxd::{Coin as BackendCoin, CosmWasmCoin};

const MINOR_IN_MAJOR: f64 = 1_000_000.;

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export, export_to = "../src/types/rust/denom.ts"))]
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum Denom {
    Major,
    Minor,
}

impl FromStr for Denom {
    type Err = BackendError;

    fn from_str(s: &str) -> Result<Denom, BackendError> {
        let s = s.to_lowercase();
        for network in Network::iter() {
            let denom = network.denom();
            if s == denom.to_lowercase() || s == "minor" {
                return Ok(Denom::Minor);
            } else if s == denom[1..].to_lowercase() || s == "major" {
                return Ok(Denom::Major);
            }
        }
        Err(BackendError::InvalidDenom(s))
    }
}

impl TryFrom<CosmosDenom> for Denom {
    type Error = BackendError;

    fn try_from(value: CosmosDenom) -> Result<Self, Self::Error> {
        Denom::from_str(&value.to_string())
    }
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/coin.ts"))]
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Coin {
    amount: String,
    denom: Denom,
}

impl Coin {
    #[allow(unused)]
    pub fn major<T: ToString>(amount: T) -> Coin {
        Coin {
            amount: amount.to_string(),
            denom: Denom::Major,
        }
    }

    #[allow(unused)]
    pub fn minor<T: ToString>(amount: T) -> Coin {
        Coin {
            amount: amount.to_string(),
            denom: Denom::Minor,
        }
    }

    pub fn new<T: ToString>(amount: T, denom: &Denom) -> Coin {
        Coin {
            amount: amount.to_string(),
            denom: denom.clone(),
        }
    }

    pub fn to_major(&self) -> Coin {
        match self.denom {
            Denom::Major => self.clone(),
            Denom::Minor => Coin {
                amount: (self.amount.parse::<f64>().unwrap() / MINOR_IN_MAJOR).to_string(),
                denom: Denom::Major,
            },
        }
    }

    pub fn to_minor(&self) -> Coin {
        match self.denom {
            Denom::Minor => self.clone(),
            Denom::Major => Coin {
                amount: (self.amount.parse::<f64>().unwrap() * MINOR_IN_MAJOR).to_string(),
                denom: Denom::Minor,
            },
        }
    }

    pub fn amount(&self) -> String {
        self.amount.clone()
    }

    #[allow(unused)]
    pub fn denom(&self) -> Denom {
        self.denom.clone()
    }

    // Helper function that returns the local denom in terms of the specified network denom.
    fn denom_as_string(&self, network_denom: &str) -> Result<String, BackendError> {
        // Currently there is the widespread assumption that network denomination is always in
        // `Denom::Minor`, and starts with 'u'.
        let network_denom = network_denom.to_owned();
        if !network_denom.starts_with('u') {
            return Err(BackendError::InvalidNetworkDenom(network_denom));
        }

        Ok(match &self.denom {
            Denom::Minor => network_denom,
            Denom::Major => network_denom[1..].to_string(),
        })
    }

    pub fn into_backend_coin(self, network_denom: &str) -> Result<BackendCoin, BackendError> {
        Ok(BackendCoin::new(
            self.amount.parse()?,
            self.denom_as_string(network_denom)?,
        ))
    }
}

impl From<BackendCoin> for Coin {
    fn from(c: BackendCoin) -> Self {
        Coin {
            amount: c.amount.to_string(),
            denom: Denom::from_str(c.denom.as_ref()).unwrap(),
        }
    }
}

impl From<CosmosCoin> for Coin {
    fn from(c: CosmosCoin) -> Coin {
        Coin {
            amount: c.amount.to_string(),
            denom: Denom::from_str(c.denom.as_ref()).unwrap(),
        }
    }
}

impl From<CosmWasmCoin> for Coin {
    fn from(c: CosmWasmCoin) -> Coin {
        Coin {
            amount: c.amount.to_string(),
            denom: Denom::from_str(&c.denom).unwrap(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::error::BackendError;
    use serde_json::json;

    use std::str::FromStr;

    #[test]
    fn json_to_coin() {
        let minor = json!({
            "amount": "1",
            "denom": "Minor"
        });

        let major = json!({
            "amount": "1",
            "denom": "Major"
        });

        let test_minor_coin = Coin::minor("1");
        let test_major_coin = Coin::major("1");

        let minor_coin = serde_json::from_value::<Coin>(minor).unwrap();
        let major_coin = serde_json::from_value::<Coin>(major).unwrap();

        assert_eq!(minor_coin, test_minor_coin);
        assert_eq!(major_coin, test_major_coin);
    }

    #[test]
    fn denom_from_str() {
        assert_eq!(Denom::from_str("unym").unwrap(), Denom::Minor);
        assert_eq!(Denom::from_str("nym").unwrap(), Denom::Major);
        assert_eq!(Denom::from_str("minor").unwrap(), Denom::Minor);
        assert_eq!(Denom::from_str("major").unwrap(), Denom::Major);

        assert!(matches!(
            Denom::from_str("foo").unwrap_err(),
            BackendError::InvalidDenom { .. },
        ));
    }

    #[test]
    fn denom_conversions() {
        let minor = Coin::minor("1");
        let major = minor.to_major();

        assert_eq!(major, Coin::major("0.000001"));

        let minor = major.to_minor();
        assert_eq!(minor, Coin::minor("1"));
    }

    #[test]
    fn network_denom_is_assumed_to_be_in_minor_denom() {
        let network_denom = "nym";
        assert!(matches!(
            Coin::minor("42")
                .denom_as_string(network_denom)
                .unwrap_err(),
            BackendError::InvalidNetworkDenom { .. }
        ));
    }

    #[test]
    fn local_denom_to_interpreted_using_network_denom() {
        let network_denom = "unym";
        assert_eq!(
            Coin::minor("42").denom_as_string(network_denom).unwrap(),
            "unym",
        );
        assert_eq!(
            Coin::major("42").denom_as_string(network_denom).unwrap(),
            "nym",
        );
    }

    #[test]
    fn coin_to_coin_minor() {
        let network_denom = "unym";
        let coin = Coin::minor("42");

        let backend_coin = coin.into_backend_coin(network_denom).unwrap();
        assert_eq!(backend_coin, BackendCoin::new(42, "unym"));
    }

    #[test]
    fn coin_to_coin_major() {
        let network_denom = "unym";
        let coin = Coin::major("52");

        let backend_coin = coin.into_backend_coin(network_denom).unwrap();
        assert_eq!(backend_coin, BackendCoin::new(52, "nym"));
    }

    fn amounts() -> Vec<&'static str> {
        vec![
            "1",
            "10",
            "100",
            "1000",
            "10000",
            "100000",
            "10000000",
            "100000000",
            "1000000000",
            "10000000000",
            "100000000000",
            "1000000000000",
            "10000000000000",
            "100000000000000",
            "1000000000000000",
            "10000000000000000",
            "100000000000000000",
            "1000000000000000000",
        ]
    }

    #[test]
    fn coin_to_backend() {
        let network_denom = "unym";
        for amount in amounts() {
            let coin = Coin::minor(amount);
            let backend_coin = coin.into_backend_coin(network_denom).unwrap();
            assert_eq!(
                backend_coin,
                BackendCoin::new(amount.parse::<u128>().unwrap(), "unym")
            );
            assert_eq!(Coin::try_from(backend_coin).unwrap(), Coin::minor(amount));

            let coin = Coin::major(amount);
            let backend_coin = coin.into_backend_coin(network_denom).unwrap();
            assert_eq!(
                backend_coin,
                BackendCoin::new(amount.parse::<u128>().unwrap(), "nym")
            );
            assert_eq!(Coin::try_from(backend_coin).unwrap(), Coin::major(amount));
        }
    }
}
