// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::MAX_RANGE_SIZE;
use std::cmp::min;
use std::collections::VecDeque;
use std::ops::Range;

pub(crate) fn split_request_range(request_range: Range<u32>) -> VecDeque<Range<u32>> {
    let mut requests = VecDeque::new();

    let mut start = request_range.start;
    let mut end = min(request_range.end, start + MAX_RANGE_SIZE as u32);

    loop {
        requests.push_back(start..end);
        start = min(start + MAX_RANGE_SIZE as u32, request_range.end);
        end = min(end + MAX_RANGE_SIZE as u32, request_range.end);

        if start == end {
            break;
        }
    }

    requests
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitting_request_range() {
        let range = 0..100;
        let mut expected = VecDeque::new();
        expected.push_back(0..30);
        expected.push_back(30..60);
        expected.push_back(60..90);
        expected.push_back(90..100);
        assert_eq!(expected, split_request_range(range));

        let range = 0..30;
        let mut expected = VecDeque::new();
        expected.push_back(0..30);
        assert_eq!(expected, split_request_range(range));

        let range = 0..60;
        let mut expected = VecDeque::new();
        expected.push_back(0..30);
        expected.push_back(30..60);
        assert_eq!(expected, split_request_range(range));

        let range = 0..5;
        let mut expected = VecDeque::new();
        expected.push_back(0..5);
        assert_eq!(expected, split_request_range(range));

        let range = 123..200;
        let mut expected = VecDeque::new();
        expected.push_back(123..153);
        expected.push_back(153..183);
        expected.push_back(183..200);
        assert_eq!(expected, split_request_range(range));
    }
}
