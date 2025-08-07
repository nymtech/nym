// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub(super) fn request_sizes(total: usize, max_request_size: usize) -> impl Iterator<Item = usize> {
    (0..total)
        .step_by(max_request_size)
        .map(move |start| std::cmp::min(max_request_size, total - start))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_sizes_test() {
        assert_eq!(
            request_sizes(100, 32).collect::<Vec<_>>(),
            vec![32, 32, 32, 4]
        );

        assert_eq!(request_sizes(10, 32).collect::<Vec<_>>(), vec![10]);
        assert_eq!(request_sizes(32, 32).collect::<Vec<_>>(), vec![32]);
        assert_eq!(request_sizes(33, 32).collect::<Vec<_>>(), vec![32, 1]);
        assert_eq!(request_sizes(1, 32).collect::<Vec<_>>(), vec![1]);
    }
}
