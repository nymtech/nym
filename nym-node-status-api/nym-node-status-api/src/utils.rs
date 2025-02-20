use cosmwasm_std::Decimal;
use itertools::Itertools;
use rand::prelude::SliceRandom;
use rand::SeedableRng;

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

#[cfg(test)]
mod test {
    use rand::Rng;

    use super::*;

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
