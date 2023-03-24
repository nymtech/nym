// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bytes::{BufMut, Bytes, BytesMut};
use futures::Stream;
use log::error;
use std::cell::RefCell;
use std::future::Future;
use std::io;
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncRead;
use tokio::time::{sleep, Duration, Instant, Sleep};
use tokio_util::io::poll_read_buf;

const MAX_READ_AMOUNT: usize = 500 * 1000; // 0.5MB
const GRACE_DURATION: Duration = Duration::from_millis(1);
const READ_TIMEOUT: Duration = Duration::from_millis(10);

pub struct AvailableReader<'a, R: AsyncRead + Unpin> {
    // TODO: come up with a way to avoid using RefCell (not sure if possible though due to having to
    // mutably borrow both inner reader and buffer at the same time)
    buf: RefCell<BytesMut>,
    inner: RefCell<&'a mut R>,
    grace_period: Option<Pin<Box<Sleep>>>,
    read_deadline: Option<Pin<Box<Sleep>>>,
}

impl<'a, R> AvailableReader<'a, R>
where
    R: AsyncRead + Unpin,
{
    const BUF_INCREMENT: usize = 4096;

    pub fn new(reader: &'a mut R) -> Self {
        // pub fn new(reader: &'a mut R, buffer_size: usize) -> Self {
        AvailableReader {
            buf: RefCell::new(BytesMut::with_capacity(Self::BUF_INCREMENT)),
            inner: RefCell::new(reader),
            grace_period: None,
            read_deadline: None,
        }
    }

    fn return_buf(mut self: Pin<&mut Self>) -> Poll<Option<<Self as Stream>::Item>> {
        // reset timeouts so the poll wouldn't be accidentally called
        self.grace_period = None;
        self.read_deadline = None;

        let buf = self.buf.borrow_mut().split();
        // there's no data in the buffer, it means the underlying source is done
        if buf.is_empty() {
            Poll::Ready(None)
        } else {
            Poll::Ready(Some(Ok(buf.freeze())))
        }
    }

    fn poll_read_deadline(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        match self.read_deadline.as_mut() {
            Some(read_deadline) => Pin::new(read_deadline).poll(cx),
            None => {
                // this is the first time we're polling this reader - set the deadline
                self.read_deadline = Some(Box::pin(sleep(READ_TIMEOUT)));
                Poll::Pending
            }
        }
    }

    fn poll_grace_period(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        match self.grace_period.as_mut() {
            Some(grace_period) => Pin::new(grace_period).poll(cx),
            None => {
                // this is the first time we're polling this reader - set the grace period
                self.grace_period = Some(Box::pin(sleep(GRACE_DURATION)));
                Poll::Pending
            }
        }
    }

    fn reset_grace_period(&mut self) {
        if let Some(grace_period) = self.grace_period.as_mut() {
            let now = Instant::now();
            grace_period.as_mut().reset(now + GRACE_DURATION);
        } else {
            // this branch should be impossible!
            error!("reached an impossible branch - attempted to reset a non-existent grace period timeout")
        }
    }
}

impl<'a, R: AsyncRead + Unpin> Stream for AvailableReader<'a, R> {
    type Item = io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // if we have no space in buffer left - expand it
        if !self.buf.borrow().has_remaining_mut() {
            self.buf.borrow_mut().reserve(Self::BUF_INCREMENT);
        }

        // note: poll_read_buf calls `buf.advance_mut(n)`
        let poll_res = poll_read_buf(
            Pin::new(self.inner.borrow_mut().deref_mut()),
            cx,
            self.buf.borrow_mut().deref_mut(),
        );

        let deadline_poll_res = self.poll_read_deadline(cx);
        let grace_period_poll_res = self.poll_grace_period(cx);

        match poll_res {
            Poll::Pending => {
                // there's nothing for us here, just return whatever we have
                // (assuming we read anything and we SHOULD HAVE read something)
                if self.buf.borrow().is_empty() {
                    // this case shouldn't be possible - if something woke the future we MUST HAVE HAD some data
                    // (it's because we never set the sleeps until the first call, meaning some data was already there)
                    error!("AvailableReader got woken with not data to return!");
                    Poll::Pending
                } else if grace_period_poll_res.is_pending() && deadline_poll_res.is_pending() {
                    // if neither timeout was reached yet, wait a bit more
                    Poll::Pending
                } else {
                    // otherwise return what we managed to read (and reset timeouts)
                    self.return_buf()
                }
            }
            Poll::Ready(Err(err)) => Poll::Ready(Some(Err(err))),
            Poll::Ready(Ok(n)) => {
                self.reset_grace_period();

                if n == 0 {
                    // if we read 0 bytes, it means the underlying source is done, so return what we have
                    // (note, this might get called twice if we already had something in the buffer)
                    self.return_buf()
                } else {
                    // tell the waker we should be polled again!
                    cx.waker().wake_by_ref();

                    // if we reached our maximum amount or we've been trying to read the data for too long
                    // return what we have
                    let read_bytes_len = self.buf.borrow().len();
                    if read_bytes_len >= MAX_READ_AMOUNT || deadline_poll_res.is_ready() {
                        self.return_buf()
                    } else {
                        Poll::Pending
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{poll, StreamExt};
    use std::io::Cursor;
    use std::time::Duration;
    use tokio::io::AsyncReadExt;
    use tokio_test::assert_pending;

    #[tokio::test]
    async fn available_reader_reads_all_available_data_smaller_than_its_buf() {
        let data = vec![42u8; 100];
        let mut reader = Cursor::new(data.clone());

        let mut available_reader = AvailableReader::new(&mut reader);
        let read_data = available_reader.next().await.unwrap().unwrap();

        assert_eq!(read_data, data);
        assert!(available_reader.next().await.is_none());
    }

    #[tokio::test]
    async fn available_reader_reads_all_available_data_bigger_than_its_buf() {
        let data = vec![42u8; AvailableReader::<Cursor<Vec<u8>>>::BUF_INCREMENT + 100];
        let mut reader = Cursor::new(data.clone());

        let mut available_reader = AvailableReader::new(&mut reader);
        let read_data = available_reader.next().await.unwrap().unwrap();

        assert_eq!(read_data, data);
        assert!(available_reader.next().await.is_none());
    }

    #[tokio::test]
    async fn available_reader_will_not_wait_for_more_data_if_it_already_has_some() {
        let first_data_chunk = vec![42u8; 100];
        let second_data_chunk = vec![123u8; 100];

        let mut reader_mock = tokio_test::io::Builder::new()
            .read(&first_data_chunk)
            .wait(Duration::from_millis(100)) // delay is irrelevant, what matters is that we don't get everything immediately
            .read(&second_data_chunk)
            .build();

        let mut available_reader = AvailableReader::new(&mut reader_mock);
        let read_data = available_reader.next().await.unwrap().unwrap();

        assert_eq!(read_data, first_data_chunk);
        assert_pending!(poll!(available_reader.next()));
        // before dropping the mock, we need to empty it
        let mut buf = vec![0u8; second_data_chunk.len()];
        assert_eq!(reader_mock.read(&mut buf).await.unwrap(), 100);
    }

    #[tokio::test]
    async fn available_reader_will_wait_for_more_data_if_it_doesnt_have_anything() {
        let data = vec![42u8; 100];

        let mut reader_mock = tokio_test::io::Builder::new()
            .wait(Duration::from_millis(100))
            .read(&data)
            .build();

        let mut available_reader = AvailableReader::new(&mut reader_mock);
        let read_data = available_reader.next().await.unwrap().unwrap();

        assert_eq!(read_data, data);
        assert!(available_reader.next().await.is_none());
    }

    // perhaps the issue of tokio io builder will be resolved in tokio 0.3?
    // #[tokio::test]
    // async fn available_reader_will_wait_for_more_data_if_its_within_grace_period() {
    //     let first_data_chunk = vec![42u8; 100];
    //     let second_data_chunk = vec![123u8; 100];
    //
    //     let combined_chunks: Vec<_> = first_data_chunk
    //         .iter()
    //         .cloned()
    //         .chain(second_data_chunk.iter().cloned())
    //         .collect();
    //
    //     let mut reader_mock = tokio_test::io::Builder::new()
    //         .read(&first_data_chunk)
    //         .wait(Duration::from_millis(2))
    //         .read(&second_data_chunk)
    //         .build();
    //
    //     let mut available_reader = AvailableReader {
    //         buf: RefCell::new(BytesMut::with_capacity(4096)),
    //         inner: RefCell::new(&mut reader_mock),
    //         grace_period: Some(delay_for(Duration::from_millis(5))),
    //     };
    //
    //     let read_data = available_reader.next().await.unwrap().unwrap();
    //
    //     assert_eq!(read_data, combined_chunks);
    //     assert!(available_reader.next().await.is_none())
    // }
}
