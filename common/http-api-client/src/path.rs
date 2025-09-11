// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

/// Collection of URL Path Segments
pub type PathSegments<'a> = &'a [&'a str];

fn sanitize_fragment(segment: &str) -> &str {
    segment.trim_matches(|c: char| c.is_whitespace() || c == '/')
}

pub trait RequestPath: Debug {
    fn to_sanitized_segments(&self) -> Vec<&str>;
}

macro_rules! impl_stringified_sanitized_segments {
    ($frag_iter:expr) => {{
        let mut path_segments = Vec::new();

        for segment in $frag_iter {
            if !segment.is_empty() {
                path_segments.push(sanitize_fragment(segment));
            }
        }

        path_segments
    }};
}

impl RequestPath for PathSegments<'_> {
    fn to_sanitized_segments(&self) -> Vec<&str> {
        impl_stringified_sanitized_segments!(self.iter())
    }
}

impl<const N: usize> RequestPath for &[&str; N] {
    fn to_sanitized_segments(&self) -> Vec<&str> {
        impl_stringified_sanitized_segments!(self.iter())
    }
}

impl RequestPath for &str {
    fn to_sanitized_segments(&self) -> Vec<&str> {
        impl_stringified_sanitized_segments!(self.split('/'))
    }
}

impl RequestPath for String {
    fn to_sanitized_segments(&self) -> Vec<&str> {
        impl_stringified_sanitized_segments!(self.split('/'))
    }
}

impl RequestPath for &String {
    fn to_sanitized_segments(&self) -> Vec<&str> {
        impl_stringified_sanitized_segments!(self.split('/'))
    }
}
