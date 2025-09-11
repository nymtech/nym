use cosmwasm_std::Decimal;
use itertools::Itertools;
use rand::prelude::SliceRandom;
use rand::SeedableRng;
use tracing::error;

// pub(crate) fn generate_node_name(identity: ed25519::PublicKey) -> String {
pub(crate) fn generate_node_name(node_id: i64) -> String {
    let seed = {
        let node_id_bytes = node_id.to_le_bytes();
        let mut seed = [0u8; 32];
        for i in 0..4 {
            seed[i * 8..(i + 1) * 8].copy_from_slice(&node_id_bytes);
        }
        seed
    };
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(seed);
    let words = bip39::Language::English.word_list();
    words.choose_multiple(&mut rng, 3).join(" ")
}

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod test {
    use super::*;
    use rand::Rng;
    use std::str::FromStr;

    #[test]
    fn generate_node_name_should_be_deterministic() {
        let mut rng = rand::thread_rng();

        let node_id: i64 = rng.gen();
        let different_node_id: i64 = rng.gen();

        let node_name = generate_node_name(node_id);
        let node_name_different = generate_node_name(different_node_id);
        assert_ne!(node_name, node_name_different);

        let node_name_same = generate_node_name(node_id);
        assert_eq!(node_name, node_name_same);
    }

    #[test]
    fn test_decimal_to_i64() {
        // Test with a simple decimal
        let dec1 = Decimal::from_str("123.456").unwrap();
        assert_eq!(decimal_to_i64(dec1), 123);

        // Test with a decimal that has more than 6 decimal places
        let dec2 = Decimal::from_str("123.456789").unwrap();
        assert_eq!(decimal_to_i64(dec2), 123);

        // Test with a decimal that rounds up
        let dec3 = Decimal::from_str("123.9999999").unwrap();
        assert_eq!(decimal_to_i64(dec3), 124);

        // Test with a zero decimal
        let dec4 = Decimal::zero();
        assert_eq!(decimal_to_i64(dec4), 0);

        // Test with a large decimal
        let dec5 = Decimal::from_str("1234567890.123456").unwrap();
        assert_eq!(decimal_to_i64(dec5), 1234567890);
    }

    #[test]
    fn test_unix_timestamp_to_utc_rfc3339() {
        // Test with a known timestamp
        let ts1 = 1672531199; // 2022-12-31 23:59:59 UTC
        assert_eq!(unix_timestamp_to_utc_rfc3339(ts1), "2022-12-31T23:59:59Z");

        // Test with the Unix epoch
        let ts2 = 0;
        assert_eq!(unix_timestamp_to_utc_rfc3339(ts2), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn test_numerical_checked_cast() {
        // Test successful cast
        let val1: u32 = 123;
        let res1: anyhow::Result<u64> = val1.cast_checked();
        assert_eq!(res1.unwrap(), 123u64);

        // Test failing cast
        let val2: i64 = -1;
        let res2: anyhow::Result<u32> = val2.cast_checked();
        assert!(res2.is_err());
    }
}

pub(crate) fn now_utc() -> time::UtcDateTime {
    time::UtcDateTime::now()
}

pub(crate) fn unix_timestamp_to_utc_rfc3339(unix_timestamp: i64) -> String {
    let timestamp = time::UtcDateTime::UNIX_EPOCH + time::Duration::seconds(unix_timestamp);
    timestamp
        .format(&time::format_description::well_known::Rfc3339)
        // unwrap: time-rs guarantees that output will be valid according to spec
        // https://time-rs.github.io/book/api/well-known-format-descriptions.html
        .unwrap_or_else(|e| {
            error!("Formatting {} as RFC3339 failed: {}", timestamp, e);
            String::from("invalid_date")
        })
}

pub trait NumericalCheckedCast<T>
where
    T: TryFrom<Self>,
    <T as TryFrom<Self>>::Error: std::error::Error,
    Self: std::fmt::Display + Copy,
{
    fn cast_checked(self) -> anyhow::Result<T> {
        T::try_from(self).map_err(|e| {
            anyhow::anyhow!(
                "Couldn't cast {} to {}: {}",
                self,
                std::any::type_name::<T>(),
                e
            )
        })
    }
}

impl<T, U> NumericalCheckedCast<U> for T
where
    U: TryFrom<T>,
    <U as TryFrom<T>>::Error: std::error::Error,
    T: std::fmt::Display + Copy,
{
}

pub(crate) fn decimal_to_i64(decimal: Decimal) -> i64 {
    // Convert the underlying Uint128 to a u128
    let atomics = decimal.atomics().u128();
    let precision = 1_000_000_000_000_000_000u128;

    // Get the fractional part
    let fractional = atomics % precision;

    // Get the integer part
    let integer = atomics / precision;

    // Combine them into a float
    let float_value = integer as f64 + (fractional as f64 / 1_000_000_000_000_000_000_f64);

    // Limit to 6 decimal places
    let rounded_value = (float_value * 1_000_000.0).round() / 1_000_000.0;

    rounded_value as i64
}

pub(crate) trait LogError<T, E> {
    fn log_error(self, msg: &str) -> Result<T, E>;
}

impl<T, E> LogError<T, E> for anyhow::Result<T, E>
where
    E: std::error::Error,
{
    fn log_error(self, msg: &str) -> Result<T, E> {
        if let Err(e) = &self {
            tracing::error!("[{msg}]:\t{e}");
        }
        self
    }
}
