// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use rand::seq::{index, WeightError};
use rand::Rng;

/// Weighted sampling without replacement where zero-weight items are last-resort filler:
/// they are only returned if the positive-weight items alone cannot satisfy `amount`.
pub(crate) fn sample_weighted_with_filler<'a, T, R, F>(
    rng: &mut R,
    items: &'a [T],
    amount: usize,
    weight: F,
) -> Result<Vec<&'a T>, WeightError>
where
    R: Rng + ?Sized,
    F: Fn(&T) -> f64,
{
    let amount = amount.min(items.len());
    let weighted = index::sample_weighted(rng, items.len(), |idx| weight(&items[idx]), amount)?;

    let mut sampled = weighted
        .into_iter()
        .map(|idx| &items[idx])
        .collect::<Vec<_>>();

    // the weighted pass returns every positive-weight item before it comes up short,
    // so whatever it skipped has zero weight: draw the remainder from those, uniformly
    let shortfall = amount - sampled.len();
    if shortfall == 0 {
        return Ok(sampled);
    }

    let filler = items
        .iter()
        .filter(|item| weight(item) <= 0.)
        .collect::<Vec<_>>();
    for idx in index::sample(rng, filler.len(), shortfall.min(filler.len())) {
        sampled.push(filler[idx])
    }

    Ok(sampled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    fn weight(item: &(&str, f64)) -> f64 {
        item.1
    }

    fn names<'a>(sampled: Vec<&'a (&'a str, f64)>) -> Vec<&'a str> {
        sampled.into_iter().map(|item| item.0).collect()
    }

    #[test]
    fn zero_weight_items_fill_the_shortfall() {
        let items = [("a", 1.0), ("b", 0.0), ("c", 0.0)];
        let mut rng = ChaCha20Rng::seed_from_u64(42);

        let mut sampled = names(sample_weighted_with_filler(&mut rng, &items, 3, weight).unwrap());
        sampled.sort();

        assert_eq!(vec!["a", "b", "c"], sampled);
    }

    #[test]
    fn all_zero_weights_still_yield_a_full_sample() {
        let items = [("a", 0.0), ("b", 0.0), ("c", 0.0)];
        let mut rng = ChaCha20Rng::seed_from_u64(42);

        let sampled = sample_weighted_with_filler(&mut rng, &items, 2, weight).unwrap();

        assert_eq!(2, sampled.len());
    }

    #[test]
    fn zero_weight_items_are_not_used_while_positive_ones_remain() {
        let items = [("a", 1.0), ("b", 2.0), ("c", 0.0)];

        for seed in 0..50 {
            let mut rng = ChaCha20Rng::seed_from_u64(seed);
            let sampled = names(sample_weighted_with_filler(&mut rng, &items, 2, weight).unwrap());
            assert!(
                !sampled.contains(&"c"),
                "zero-weight item chosen: {sampled:?}"
            );
        }
    }

    #[test]
    fn requesting_more_than_available_returns_everything() {
        let items = [("a", 1.0), ("b", 0.0)];
        let mut rng = ChaCha20Rng::seed_from_u64(42);

        let sampled = sample_weighted_with_filler(&mut rng, &items, 10, weight).unwrap();

        assert_eq!(2, sampled.len());
    }

    #[test]
    fn negative_weights_are_rejected() {
        let items = [("a", 1.0), ("b", -1.0)];
        let mut rng = ChaCha20Rng::seed_from_u64(42);

        let res = sample_weighted_with_filler(&mut rng, &items, 1, weight);

        assert!(matches!(res, Err(WeightError::InvalidWeight)));
    }
}
