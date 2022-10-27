// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::TOTAL_DEALINGS;
use std::fmt::Debug;

pub(crate) fn transpose_matrix<T: Debug>(
    matrix: Vec<[T; TOTAL_DEALINGS]>,
) -> [Vec<T>; TOTAL_DEALINGS] {
    let mut iters: Vec<_> = matrix.into_iter().map(|d| d.into_iter()).collect();
    (0..TOTAL_DEALINGS)
        .map(|_| {
            iters
                .iter_mut()
                .map(|it| it.next().unwrap())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiple_sizes() {
        let empty: Vec<[f32; TOTAL_DEALINGS]> = vec![];
        let small: Vec<[usize; TOTAL_DEALINGS]> = vec![
            [312usize; TOTAL_DEALINGS],
            [4usize; TOTAL_DEALINGS],
            [12usize; TOTAL_DEALINGS],
            [123usize; TOTAL_DEALINGS],
        ];
        let big: Vec<[i32; TOTAL_DEALINGS]> = (0..1000)
            .map(|idx| [idx * 5 - 42; TOTAL_DEALINGS])
            .collect();

        assert_eq!(
            transpose_matrix(empty),
            core::array::from_fn::<Vec<f32>, TOTAL_DEALINGS, _>(|_| vec![])
        );
        assert_eq!(
            transpose_matrix(small),
            core::array::from_fn::<Vec<usize>, TOTAL_DEALINGS, _>(|_| vec![312, 4, 12, 123])
        );
        assert_eq!(
            transpose_matrix(big),
            core::array::from_fn::<Vec<i32>, TOTAL_DEALINGS, _>(|_| (0..1000)
                .map(|idx| idx as i32 * 5 - 42)
                .collect())
        );
    }
}
