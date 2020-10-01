// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bytes::{BufMut, Bytes, BytesMut};
use std::cell::RefCell;
use std::future::Future;
use std::io;
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncRead;
use tokio::stream::Stream;
use tokio::time::Delay;

pub struct AvailableReader<'a, R: AsyncRead + Unpin> {
    // TODO: come up with a way to avoid using RefCell (not sure if possible though)
    buf: RefCell<BytesMut>,
    inner: RefCell<&'a mut R>,
    // idea for the future: tiny delay that allows to prevent unnecessary extra fragmentation
    // grace_period: Option<Delay>,
}

impl<'a, R> AvailableReader<'a, R>
where
    R: AsyncRead + Unpin,
{
    const BUF_INCREMENT: usize = 4096;

    pub fn new(reader: &'a mut R) -> Self {
        AvailableReader {
            buf: RefCell::new(BytesMut::with_capacity(Self::BUF_INCREMENT)),
            inner: RefCell::new(reader),
            // grace_period: None,
        }
    }
}

// TODO: change this guy to a stream? Seems waaay more appropriate considering
// we're getting new Bytes items regularly rather than calling it once.

impl<'a, R: AsyncRead + Unpin> Stream for AvailableReader<'a, R> {
    // todo: remove bool by being able to infer from the option
    type Item = io::Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // if we have no space in buffer left - expand it
        if !self.buf.borrow().has_remaining_mut() {
            self.buf.borrow_mut().reserve(Self::BUF_INCREMENT);
        }

        // note: poll_read_buf calls `buf.advance_mut(n)`
        let poll_res = Pin::new(self.inner.borrow_mut().deref_mut())
            .poll_read_buf(cx, self.buf.borrow_mut().deref_mut());

        match poll_res {
            Poll::Pending => {
                // there's nothing for us here, just return whatever we have (assuming we read anything!)
                if self.buf.borrow().is_empty() {
                    Poll::Pending
                } else {
                    let buf = self.buf.replace(BytesMut::new());
                    Poll::Ready(Some(Ok(buf.freeze())))
                }
            }
            Poll::Ready(Err(err)) => Poll::Ready(Some(Err(err))),
            Poll::Ready(Ok(n)) => {
                // if we read a non-0 amount, we're not done yet!
                if n == 0 {
                    let buf = self.buf.replace(BytesMut::new());
                    if buf.len() > 0 {
                        Poll::Ready(Some(Ok(buf.freeze())))
                    } else {
                        Poll::Ready(None)
                    }
                } else {
                    // tell the waker we should be polled again!
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }
    }
}
//
// impl<'a, R: AsyncRead + Unpin> Future for AvailableReader<'a, R> {
//     type Output = io::Result<(Bytes, bool)>;
//
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         // if we have no space in buffer left - expand it
//         if !self.buf.borrow().has_remaining_mut() {
//             self.buf.borrow_mut().reserve(Self::BUF_INCREMENT);
//         }
//
//         // note: poll_read_buf calls `buf.advance_mut(n)`
//         let poll_res = Pin::new(self.inner.borrow_mut().deref_mut())
//             .poll_read_buf(cx, self.buf.borrow_mut().deref_mut());
//
//         match poll_res {
//             Poll::Pending => {
//                 // there's nothing for us here, just return whatever we have (assuming we read anything!)
//                 if self.buf.borrow().is_empty() {
//                     Poll::Pending
//                 } else {
//                     let buf = self.buf.replace(BytesMut::new());
//                     Poll::Ready(Ok((buf.freeze(), false)))
//                 }
//             }
//             Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
//             Poll::Ready(Ok(n)) => {
//                 // if we read a non-0 amount, we're not done yet!
//                 if n == 0 {
//                     let buf = self.buf.replace(BytesMut::new());
//                     Poll::Ready(Ok((buf.freeze(), true)))
//                 } else {
//                     // tell the waker we should be polled again!
//                     cx.waker().wake_by_ref();
//                     Poll::Pending
//                 }
//             }
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use futures::poll;
    use std::io::Cursor;
    use std::time::Duration;
    use tokio::stream::StreamExt;
    use tokio_test::assert_pending;

    #[tokio::test]
    async fn available_reader_reads_all_available_data_smaller_than_its_buf() {
        let data = vec![42u8; 100];
        let mut reader = Cursor::new(data.clone());

        let mut available_reader = AvailableReader::new(&mut reader);
        let read_data = available_reader.next().await.unwrap().unwrap();

        assert_eq!(read_data, data);
        assert!(available_reader.next().await.is_none())
    }

    #[tokio::test]
    async fn available_reader_reads_all_available_data_bigger_than_its_buf() {
        let data = vec![42u8; AvailableReader::<Cursor<Vec<u8>>>::BUF_INCREMENT + 100];
        let mut reader = Cursor::new(data.clone());

        let mut available_reader = AvailableReader::new(&mut reader);
        let read_data = available_reader.next().await.unwrap().unwrap();

        assert_eq!(read_data, data);
        assert!(available_reader.next().await.is_none())
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
        assert!(available_reader.next().await.is_none())
    }
}
