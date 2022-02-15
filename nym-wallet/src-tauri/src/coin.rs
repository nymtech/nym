// This should be moved out of the wallet, and used as a primary coin type throughout the codebase

use crate::error::BackendError;
use crate::network::Network;
use ::config::defaults::DENOM;
use cosmrs::Decimal;
use cosmrs::Denom as CosmosDenom;
use cosmwasm_std::Coin as CosmWasmCoin;
use cosmwasm_std::Uint128;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;
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

impl fmt::Display for Denom {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match *self {
      Denom::Major => f.write_str(&DENOM[1..]),
      Denom::Minor => f.write_str(DENOM),
    }
  }
}

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

// TODO convert to TryFrom
impl From<GasPrice> for Coin {
  fn from(g: GasPrice) -> Coin {
    Coin {
      amount: g.amount.to_string(),
      denom: Denom::from_str(&g.denom.to_string()).unwrap(),
    }
  }
}

impl fmt::Display for Coin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&format!("{} {}", self.amount, self.denom))
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
  pub fn major<T: ToString>(amount: T) -> Coin {
    Coin {
      amount: amount.to_string(),
      denom: Denom::Major,
    }
  }

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

  pub fn denom(&self) -> Denom {
    self.denom.clone()
  }
}

impl TryFrom<Coin> for CosmWasmCoin {
  type Error = BackendError;

  fn try_from(coin: Coin) -> Result<CosmWasmCoin, Self::Error> {
    Ok(CosmWasmCoin::new(
      Uint128::try_from(coin.amount.as_str()).unwrap().u128(),
      coin.denom.to_string(),
    ))
  }
}

impl TryFrom<Coin> for CosmosCoin {
  type Error = BackendError;

  fn try_from(coin: Coin) -> Result<CosmosCoin, BackendError> {
    match Decimal::from_str(&coin.amount) {
      Ok(d) => Ok(CosmosCoin {
        amount: d,
        denom: CosmosDenom::from_str(&coin.denom.to_string())?,
      }),
      Err(e) => Err(e.into()),
    }
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
  use crate::coin::{Coin, Denom};
  use cosmrs::Coin as CosmosCoin;
  use cosmrs::Decimal;
  use cosmrs::Denom as CosmosDenom;
  use cosmwasm_std::Coin as CosmWasmCoin;
  use serde_json::json;
  use std::convert::{TryFrom, TryInto};
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
  fn denom_conversions() {
    let minor = Coin::minor("1");
    let major = minor.to_major();

    assert_eq!(major, Coin::major("0.000001"));

    let minor = major.to_minor();
    assert_eq!(minor, Coin::minor("1"));
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
    for amount in amounts() {
      let coin: Coin = Coin::minor(amount).into();
      let cosmoswasm_coin: CosmWasmCoin = coin.try_into().unwrap();
      assert_eq!(
        cosmoswasm_coin,
        CosmWasmCoin::new(amount.parse::<u128>().unwrap(), Denom::Minor.to_string())
      );
      assert_eq!(
        Coin::try_from(cosmoswasm_coin).unwrap(),
        Coin::minor(amount)
      );

      let coin: Coin = Coin::major(amount).into();
      let cosmoswasm_coin: CosmWasmCoin = coin.try_into().unwrap();
      assert_eq!(
        cosmoswasm_coin,
        CosmWasmCoin::new(amount.parse::<u128>().unwrap(), Denom::Major.to_string())
      );
      assert_eq!(
        Coin::try_from(cosmoswasm_coin).unwrap(),
        Coin::major(amount)
      );
    }
  }

  #[test]
  fn coin_to_cosmos() {
    for amount in amounts() {
      let coin: Coin = Coin::minor(amount).into();
      let cosmos_coin: CosmosCoin = coin.try_into().unwrap();
      assert_eq!(
        cosmos_coin,
        CosmosCoin {
          amount: Decimal::from_str(amount).unwrap(),
          denom: CosmosDenom::from_str(&Denom::Minor.to_string()).unwrap()
        }
      );
      assert_eq!(Coin::try_from(cosmos_coin).unwrap(), Coin::minor(amount));

      let coin: Coin = Coin::major(amount).into();
      let cosmos_coin: CosmosCoin = coin.try_into().unwrap();
      assert_eq!(
        cosmos_coin,
        CosmosCoin {
          amount: Decimal::from_str(amount).unwrap(),
          denom: CosmosDenom::from_str(&Denom::Major.to_string()).unwrap()
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
}
