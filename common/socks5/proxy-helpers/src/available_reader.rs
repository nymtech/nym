// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use futures::Stream;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncRead;

// note, min_capacity doesn't mean we're going to always read at least this amount of data,
// it defines the smallest allowed (by yours truly) upper bound
const MIN_CAPACITY: usize = 16 * 1024;
const DEFAULT_CAPACITY: usize = 64 * 1024;

pub struct AvailableReader<R> {
    inner: tokio_util::io::ReaderStream<R>,
}

impl<R: AsyncRead> AvailableReader<R> {
    pub fn new(reader: R, capacity: Option<usize>) -> Self {
        let capacity = capacity.unwrap_or(DEFAULT_CAPACITY).max(MIN_CAPACITY);

        AvailableReader {
            inner: tokio_util::io::ReaderStream::with_capacity(reader, capacity),
        }
    }
}

impl<R: AsyncRead + Unpin> Stream for AvailableReader<R> {
    type Item = io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}
