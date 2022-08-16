//! Active set inclusion probability simulator

use error::Error;

mod error;

const TOLERANCE_L2_NORM: f64 = 1e-4;
const TOLERANCE_MAX_NORM: f64 = 1e-3;

pub struct SelectionProbability {
    pub active_set_probability: Vec<f64>,
    pub reserve_set_probability: Vec<f64>,
    pub samples: u32,
    pub delta_l2: f64,
    pub delta_max: f64,
}

pub fn simulate_selection_probability_mixnodes(
    list_stake_for_mixnodes: &[u64],
    active_set_size: usize,
    reserve_set_size: usize,
    max_samples: u32,
) -> Result<SelectionProbability, Error> {
    // Total number of existing (registered) nodes
    let num_mixnodes = list_stake_for_mixnodes.len();

    // Cumulative stake ordered by node index
    let list_cumul = cumul_sum(list_stake_for_mixnodes);

    // The computed probabilities
    let mut active_set_probability = vec![0.0; num_mixnodes];
    let mut reserve_set_probability = vec![0.0; num_mixnodes];

    // Number sufficiently large to have a good approximation of selection probability
    let mut samples = 0;
    let mut delta_l2;
    let mut delta_max;
    let mut rng = rand::thread_rng();

    loop {
        samples += 1;
        let mut sample_active_mixnodes = Vec::new();
        let mut sample_reserve_mixnodes = Vec::new();
        let mut list_cumul_temp = list_cumul.clone();

        let active_set_probability_previous = active_set_probability.clone();

        // Select the active nodes for the epoch (hour)
        while sample_active_mixnodes.len() < active_set_size {
            let candidate = sample_candidate(&list_cumul_temp, &mut rng)?;

            if !sample_active_mixnodes.contains(&candidate) {
                sample_active_mixnodes.push(candidate);
                remove_mixnode_from_cumul_stake(candidate, &mut list_cumul_temp);
            }
        }

        // Select the reserve nodes for the epoch (hour)
        while sample_reserve_mixnodes.len() < reserve_set_size {
            let candidate = sample_candidate(&list_cumul_temp, &mut rng)?;

            if !sample_reserve_mixnodes.contains(&candidate)
                && !sample_active_mixnodes.contains(&candidate)
            {
                sample_reserve_mixnodes.push(candidate);
                remove_mixnode_from_cumul_stake(candidate, &mut list_cumul_temp);
            }
        }

        // Sum up nodes being in active or reserve set
        for active_mixnodes in sample_active_mixnodes {
            active_set_probability[active_mixnodes] += 1.0;
        }
        for reserve_mixnodes in sample_reserve_mixnodes {
            reserve_set_probability[reserve_mixnodes] += 1.0;
        }

        // Convergence critera only on active set.
        // We devide by samples to get the average, that is not really part of the delta
        // computation.
        delta_l2 = l2_diff(&active_set_probability, &active_set_probability_previous)?
            / f64::from(samples);
        delta_max = max_diff(&active_set_probability, &active_set_probability_previous)?
            / f64::from(samples);
        if samples > 10 && delta_l2 < TOLERANCE_L2_NORM && delta_max < TOLERANCE_MAX_NORM
            || samples >= max_samples
        {
            break;
        }
    }

    active_set_probability
        .iter_mut()
        .for_each(|x| *x /= f64::from(samples));
    reserve_set_probability
        .iter_mut()
        .for_each(|x| *x /= f64::from(samples));

    Ok(SelectionProbability {
        active_set_probability,
        reserve_set_probability,
        samples,
        delta_l2,
        delta_max,
    })
}

// Compute the cumulative sum
fn cumul_sum<'a>(list: impl IntoIterator<Item = &'a u64>) -> Vec<u64> {
    let mut list_cumul = Vec::new();
    let mut cumul = 0;
    for entry in list {
        cumul += entry;
        list_cumul.push(cumul);
    }
    list_cumul
}

fn sample_candidate(list_cumul: &[u64], rng: &mut rand::rngs::ThreadRng) -> Result<usize, Error> {
    use rand::distributions::{Distribution, Uniform};
    let uniform = Uniform::from(0..*list_cumul.last().ok_or(Error::EmptyListCumulStake)?);
    let r = uniform.sample(rng);

    let candidate = list_cumul
        .iter()
        .enumerate()
        .find(|(_, x)| *x >= &r)
        .ok_or(Error::SamplePointOutOfBounds)?
        .0;

    Ok(candidate)
}

// Update list of cumulative stake to reflect eliminating the picked node
fn remove_mixnode_from_cumul_stake(candidate: usize, list_cumul_stake: &mut [u64]) {
    let prob_candidate = if candidate == 0 {
        list_cumul_stake[0]
    } else {
        list_cumul_stake[candidate] - list_cumul_stake[candidate - 1]
    };

    for cumul in list_cumul_stake.iter_mut().skip(candidate) {
        *cumul -= prob_candidate;
    }
}

// Compute the difference in l2-norm
fn l2_diff(v1: &[f64], v2: &[f64]) -> Result<f64, Error> {
    if v1.len() != v2.len() {
        return Err(Error::NormDifferenceSizeArrays);
    }
    Ok(v1
        .iter()
        .zip(v2)
        .map(|(&i1, &i2)| (i1 - i2).powi(2))
        .sum::<f64>()
        .sqrt())
}

// Compute the difference in max-norm
fn max_diff(v1: &[f64], v2: &[f64]) -> Result<f64, Error> {
    if v1.len() != v2.len() {
        return Err(Error::NormDifferenceSizeArrays);
    }
    Ok(v1
        .iter()
        .zip(v2)
        .map(|(x, y)| (x - y).abs())
        .fold(f64::NEG_INFINITY, f64::max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_cumul_sum() {
        let v = cumul_sum(&vec![1, 2, 3]);
        assert_eq!(v, &[1, 3, 6]);
    }

    #[test]
    fn remove_mixnode_from_cumul() {
        let mut cumul_stake = vec![1, 2, 3, 4, 5, 6];
        remove_mixnode_from_cumul_stake(3, &mut cumul_stake);
        assert_eq!(cumul_stake, &[1, 2, 3, 3, 4, 5]);
    }

    #[test]
    fn max_norm() {
        let v1 = vec![1.0, 2.0, 3.0];
        let v2 = vec![2.0, 4.0, -6.0];
        assert!((max_diff(&v1, &v2).unwrap() - 9.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ls_norm() {
        let v1 = vec![1.0, 2.0, 3.0];
        let v2 = vec![2.0, 3.0, -2.0];
        assert!((l2_diff(&v1, &v2).unwrap() - 5.196_152_422_706_632).abs() < 1e2 * f64::EPSILON);
    }

    // Replicate the results from the Python simulation code in https://github.com/nymtech/team-core/issues/114
    #[test]
    fn replicate_python_simulation() {
        let active_set_size = 4;
        let standby_set_size = 1;

        // this has to contain the total stake per node
        let list_mix = vec![
            100, 100, 3000, 500_000, 100, 10, 10, 10, 10, 10, 30000, 500, 200, 52345,
        ];

        let max_samples = 100_000;

        let SelectionProbability {
            active_set_probability,
            reserve_set_probability,
            samples,
            delta_l2,
            delta_max,
        } = simulate_selection_probability_mixnodes(
            &list_mix,
            active_set_size,
            standby_set_size,
            max_samples,
        )
        .unwrap();

        // These values comes from running the python simulator for a very long time
        let expected_active_set_probability = vec![
            0.025_070_8,
            0.025_073_2,
            0.744_117,
            0.999_999,
            0.025_000_2,
            0.002_524_4,
            0.002_527_8,
            0.002_528_6,
            0.002_569_6,
            0.002_513_6,
            0.994,
            0.125_482_8,
            0.050_279_8,
            0.998_313_2,
        ];
        // The same check is used in the convergence criterion, and hence should be reflected in
        // `delta_max` too.
        assert!(
            max_diff(&active_set_probability, &expected_active_set_probability).unwrap() < 1e-2
        );

        let expected_reserve_set_probability = vec![
            0.076_392_4,
            0.076_499,
            0.204_893_6,
            1e-06,
            0.076_278_8,
            0.007_720_6,
            0.007_673,
            0.007_700_2,
            0.007_669_4,
            0.007_731_2,
            0.005_789_4,
            0.368_465_6,
            0.151_537_2,
            0.001_648_6,
        ];
        assert!(
            max_diff(&reserve_set_probability, &expected_reserve_set_probability).unwrap() < 1e-2
        );

        // We converge around 20_000, add another 500 for some slack due to random values
        assert!(samples < 20_500);
        assert!(delta_l2 < TOLERANCE_L2_NORM);
        assert!(delta_max < TOLERANCE_MAX_NORM);
    }
}
