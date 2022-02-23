// This should be moved out of the wallet, and used as a primary coin type throughout the codebase

use crate::error::BackendError;
use crate::network::Network;
use cosmrs::Decimal;
use cosmrs::Denom as CosmosDenom;
use cosmwasm_std::Coin as CosmWasmCoin;
use cosmwasm_std::Uint128;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::ops::{Add, Sub};
use std::str::FromStr;
use strum::IntoEnumIterator;
use validator_client::nymd::{CosmosCoin, GasPrice};

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum Denom {
  Major,
  Minor,
}

const MINOR_IN_MAJOR: f64 = 1_000_000.;

impl FromStr for Denom {
  type Err = BackendError;

  fn from_str(s: &str) -> Result<Denom, BackendError> {
    let s = s.to_lowercase();
    for network in Network::iter() {
      let denom = network.denom();
      if s == denom.as_ref().to_lowercase() || s == "minor" {
        return Ok(Denom::Minor);
      } else if s == denom.as_ref()[1..].to_lowercase() || s == "major" {
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
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Coin {
  amount: String,
  denom: Denom,
}

impl TryFrom<GasPrice> for Coin {
  type Error = BackendError;

  fn try_from(g: GasPrice) -> Result<Self, Self::Error> {
    Ok(Coin {
      amount: g.amount.to_string(),
      denom: Denom::from_str(&g.denom.to_string())?,
    })
  }
}

// Allows adding minor and major denominations, output will have the LHS denom.
impl Add for Coin {
  type Output = Self;

  fn add(self, rhs: Self) -> Self {
    let denom = self.denom.clone();
    let lhs = self.to_minor();
    let rhs = rhs.to_minor();
    let lhs_amount = lhs.amount.parse::<u64>().unwrap();
    let rhs_amount = rhs.amount.parse::<u64>().unwrap();
    let amount = lhs_amount + rhs_amount;
    let coin = Coin {
      amount: amount.to_string(),
      denom: Denom::Minor,
    };
    match denom {
      Denom::Major => coin.to_major(),
      Denom::Minor => coin,
    }
  }
}

// Allows adding minor and major denominations, output will have the LHS denom.
impl Sub for Coin {
  type Output = Self;

  fn sub(self, rhs: Self) -> Self {
    let denom = self.denom.clone();
    let lhs = self.to_minor();
    let rhs = rhs.to_minor();
    let lhs_amount = lhs.amount.parse::<i64>().unwrap();
    let rhs_amount = rhs.amount.parse::<i64>().unwrap();
    let amount = lhs_amount - rhs_amount;
    let coin = Coin {
      amount: amount.to_string(),
      denom: Denom::Minor,
    };
    match denom {
      Denom::Major => coin.to_major(),
      Denom::Minor => coin,
    }
  }
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
  fn denom_as_string(&self, network_denom: &CosmosDenom) -> Result<String, BackendError> {
    // Currently there is the widespread assumption that network denomination is always in
    // `Denom::Minor`, and starts with 'u'.
    let network_denom = network_denom.to_string();
    if !network_denom.starts_with('u') {
      return Err(BackendError::InvalidNetworkDenom(network_denom));
    }

    Ok(match &self.denom {
      Denom::Minor => network_denom,
      Denom::Major => network_denom[1..].to_string(),
    })
  }

  pub fn into_cosmos_coin(self, network_denom: &CosmosDenom) -> Result<CosmosCoin, BackendError> {
    match Decimal::from_str(&self.amount) {
      Ok(amount) => Ok(CosmosCoin {
        amount,
        denom: CosmosDenom::from_str(&self.denom_as_string(network_denom)?)?,
      }),
      Err(e) => Err(e.into()),
    }
  }

  pub fn into_cosmwasm_coin(
    self,
    network_denom: &CosmosDenom,
  ) -> Result<CosmWasmCoin, BackendError> {
    Ok(CosmWasmCoin {
      denom: self.denom_as_string(network_denom)?,
      amount: Uint128::try_from(self.amount.as_str())?,
    })
  }
}

impl From<CosmosCoin> for Coin {
  fn from(c: CosmosCoin) -> Coin {
    Coin {
      amount: c.amount.to_string(),
      denom: Denom::from_str(&c.denom.to_string()).unwrap(),
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
  use cosmrs::Coin as CosmosCoin;
  use cosmrs::Decimal;
  use cosmrs::Denom as CosmosDenom;
  use cosmwasm_std::Coin as CosmWasmCoin;
  use serde_json::json;
  use std::convert::TryFrom;
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
    let network_denom = CosmosDenom::from_str("nym").unwrap();
    assert!(matches!(
      Coin::minor("42")
        .denom_as_string(&network_denom)
        .unwrap_err(),
      BackendError::InvalidNetworkDenom { .. }
    ));
  }

  #[test]
  fn local_denom_to_interpreted_using_network_denom() {
    let network_denom = CosmosDenom::from_str("unym").unwrap();
    assert_eq!(
      Coin::minor("42").denom_as_string(&network_denom).unwrap(),
      "unym",
    );
    assert_eq!(
      Coin::major("42").denom_as_string(&network_denom).unwrap(),
      "nym",
    );
  }

  #[test]
  fn coin_to_coin_minor() {
    let network_denom = CosmosDenom::from_str("unym").unwrap();
    let coin = Coin::minor("42");

    let cosmoswasm_coin = coin.clone().into_cosmwasm_coin(&network_denom).unwrap();
    assert_eq!(cosmoswasm_coin, CosmWasmCoin::new(42, "unym"),);

    let cosmos_coin = coin.into_cosmos_coin(&network_denom).unwrap();
    assert_eq!(
      cosmos_coin,
      CosmosCoin {
        denom: CosmosDenom::from_str("unym").unwrap(),
        amount: Decimal::from_str("42").unwrap(),
      },
    );
  }

  #[test]
  fn coin_to_coin_major() {
    let network_denom = CosmosDenom::from_str("unym").unwrap();
    let coin = Coin::major("52");

    let cosmoswasm_coin = coin.clone().into_cosmwasm_coin(&network_denom).unwrap();
    assert_eq!(cosmoswasm_coin, CosmWasmCoin::new(52, "nym"),);

    let cosmos_coin = coin.into_cosmos_coin(&network_denom).unwrap();
    assert_eq!(
      cosmos_coin,
      CosmosCoin {
        denom: CosmosDenom::from_str("nym").unwrap(),
        amount: Decimal::from_str("52").unwrap(),
      },
    );
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
  fn coin_to_cosmoswasm() {
    let network_denom = CosmosDenom::from_str("unym").unwrap();
    for amount in amounts() {
      let coin = Coin::minor(amount);
      let cosmoswasm_coin: CosmWasmCoin = coin.into_cosmwasm_coin(&network_denom).unwrap();
      assert_eq!(
        cosmoswasm_coin,
        CosmWasmCoin::new(amount.parse::<u128>().unwrap(), "unym")
      );
      assert_eq!(
        Coin::try_from(cosmoswasm_coin).unwrap(),
        Coin::minor(amount)
      );

      let coin = Coin::major(amount);
      let cosmoswasm_coin: CosmWasmCoin = coin.into_cosmwasm_coin(&network_denom).unwrap();
      assert_eq!(
        cosmoswasm_coin,
        CosmWasmCoin::new(amount.parse::<u128>().unwrap(), "nym")
      );
      assert_eq!(
        Coin::try_from(cosmoswasm_coin).unwrap(),
        Coin::major(amount)
      );
    }
  }

  #[test]
  fn coin_to_cosmos() {
    let network_denom = CosmosDenom::from_str("unym").unwrap();
    for amount in amounts() {
      let coin = Coin::minor(amount);
      let cosmos_coin: CosmosCoin = coin.into_cosmos_coin(&network_denom).unwrap();
      assert_eq!(
        cosmos_coin,
        CosmosCoin {
          amount: Decimal::from_str(amount).unwrap(),
          denom: CosmosDenom::from_str("unym").unwrap()
        }
      );
      assert_eq!(Coin::try_from(cosmos_coin).unwrap(), Coin::minor(amount));

      let coin = Coin::major(amount);
      let cosmos_coin: CosmosCoin = coin.into_cosmos_coin(&network_denom).unwrap();
      assert_eq!(
        cosmos_coin,
        CosmosCoin {
          amount: Decimal::from_str(amount).unwrap(),
          denom: CosmosDenom::from_str("nym").unwrap()
        }
      );
      assert_eq!(Coin::try_from(cosmos_coin).unwrap(), Coin::major(amount));
    }
  }

  #[test]
  fn test_add() {
    assert_eq!(Coin::minor("1") + Coin::minor("1"), Coin::minor("2"));
    assert_eq!(Coin::major("1") + Coin::major("1"), Coin::major("2"));
    assert_eq!(Coin::minor("1") + Coin::major("1"), Coin::minor("1000001"));
    assert_eq!(Coin::major("1") + Coin::minor("1"), Coin::major("1.000001"));
  }

  #[test]
  fn test_sub() {
    assert_eq!(Coin::minor("1") - Coin::minor("1"), Coin::minor("0"));
    assert_eq!(Coin::major("1") - Coin::major("1"), Coin::major("0"));
    assert_eq!(Coin::minor("1") - Coin::major("1"), Coin::minor("-999999"));
    assert_eq!(Coin::major("1") - Coin::minor("1"), Coin::major("0.999999"));
  }

  #[test]
  fn coin_from_gas_price() {
    assert_eq!(
      Coin::try_from(GasPrice::from_str("42unym").unwrap()).unwrap(),
      Coin {
        amount: "42".to_string(),
        denom: Denom::Minor,
      }
    );

    assert_eq!(
      Coin::try_from(GasPrice::from_str("42nym").unwrap()).unwrap(),
      Coin {
        amount: "42".to_string(),
        denom: Denom::Major,
      }
    );
  }
}
