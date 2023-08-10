use super::message::{
    ConnectionId, Message, OutboundMessage, SubstreamId, SubstreamMessage, TransportMessage,
};
use futures::{
    io::{Error as IoError, ErrorKind},
    AsyncRead, AsyncWrite,
};
use log::debug;
use nym_sphinx::addressing::clients::Recipient;
use parking_lot::Mutex;
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::{Context, Poll},
};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot::Receiver,
};

#[derive(Debug)]
pub struct Substream {
    remote_recipient: Recipient,
    connection_id: ConnectionId,
    pub(crate) substream_id: SubstreamId,

    /// inbound messages; inbound_tx is in the corresponding Connection
    pub(crate) inbound_rx: UnboundedReceiver<Vec<u8>>,

    /// outbound messages; go directly to the mixnet
    outbound_tx: UnboundedSender<OutboundMessage>,

    /// used to signal when the substream is closed
    close_rx: Receiver<()>,
    closed: Mutex<bool>,

    // buffer of data that's been written to the stream,
    // but not yet read by the application.
    unread_data: Mutex<Vec<u8>>,

    message_nonce: Arc<AtomicU64>,
}

impl Substream {
    pub(crate) fn new(
        remote_recipient: Recipient,
        connection_id: ConnectionId,
        substream_id: SubstreamId,
        inbound_rx: UnboundedReceiver<Vec<u8>>,
        outbound_tx: UnboundedSender<OutboundMessage>,
        close_rx: Receiver<()>,
        message_nonce: Arc<AtomicU64>,
    ) -> Self {
        Substream {
            remote_recipient,
            connection_id,
            substream_id,
            inbound_rx,
            outbound_tx,
            close_rx,
            closed: Mutex::new(false),
            unread_data: Mutex::new(vec![]),
            message_nonce,
        }
    }

    fn check_closed(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Result<(), IoError> {
        let closed_err = IoError::new(ErrorKind::Other, "stream closed");

        // close_rx will return an error if the channel is closed (ie. sender was dropped),
        // or if it's empty
        let received_closed = self.close_rx.try_recv();

        let mut closed = self.closed.lock();
        if *closed {
            return Err(closed_err);
        }

        if received_closed.is_ok() {
            *closed = true;
            return Err(closed_err);
        }

        Ok(())
    }
}

impl AsyncRead for Substream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, IoError>> {
        let closed_result = self.as_mut().check_closed(cx);
        if let Err(e) = closed_result {
            return Poll::Ready(Err(e));
        }

        let inbound_rx_data = self.inbound_rx.poll_recv(cx);

        // first, write any previously unread data to the buf
        let mut unread_data = self.unread_data.lock();
        let filled_len = if unread_data.len() > 0 {
            let unread_len = unread_data.len();
            let buf_len = buf.len();
            let copy_len = std::cmp::min(unread_len, buf_len);
            buf[..copy_len].copy_from_slice(&unread_data[..copy_len]);
            *unread_data = unread_data[copy_len..].to_vec();
            copy_len
        } else {
            0
        };

        if let Poll::Ready(Some(data)) = inbound_rx_data {
            if filled_len == buf.len() {
                // we've filled the buffer, so we'll have to save the rest for later
                let mut new = vec![];
                new.extend(unread_data.drain(..));
                new.extend(data.iter());
                *unread_data = new;
                return Poll::Ready(Ok(filled_len));
            }

            // otherwise, there's still room in the buffer, so we'll copy the rest of the data
            let remaining_len = buf.len() - filled_len;
            let data_len = data.len();

            // we have more data than buffer room remaining, save the extra for later
            if remaining_len < data_len {
                unread_data.extend_from_slice(&data[remaining_len..]);
            }

            let copied = std::cmp::min(remaining_len, data_len);
            buf[filled_len..filled_len + copied].copy_from_slice(&data[..copied]);
            debug!("poll_read copied {} bytes: data {:?}", copied, buf);
            return Poll::Ready(Ok(copied));
        }

        if filled_len > 0 {
            debug!("poll_read copied {} bytes: data {:?}", filled_len, buf);
            return Poll::Ready(Ok(filled_len));
        }

        Poll::Pending
    }
}

impl AsyncWrite for Substream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, IoError>> {
        if let Err(e) = self.as_mut().check_closed(cx) {
            return Poll::Ready(Err(e));
        }

        let nonce = self.message_nonce.fetch_add(1, Ordering::SeqCst);

        self.outbound_tx
            .send(OutboundMessage {
                recipient: self.remote_recipient,
                message: Message::TransportMessage(TransportMessage {
                    nonce,
                    id: self.connection_id.clone(),
                    message: SubstreamMessage::new_with_data(
                        self.substream_id.clone(),
                        buf.to_vec(),
                    ),
                }),
            })
            .map_err(|e| {
                IoError::new(
                    ErrorKind::Other,
                    format!("poll_write outbound_tx error: {}", e),
                )
            })?;

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), IoError>> {
        if let Err(e) = self.check_closed(cx) {
            return Poll::Ready(Err(e));
        }

        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), IoError>> {
        let nonce = self.message_nonce.fetch_add(1, Ordering::SeqCst);

        let mut closed = self.closed.lock();
        if *closed {
            return Poll::Ready(Err(IoError::new(ErrorKind::Other, "stream closed")));
        }

        *closed = true;

        // send a close message to the mixnet
        self.outbound_tx
            .send(OutboundMessage {
                recipient: self.remote_recipient,
                message: Message::TransportMessage(TransportMessage {
                    nonce,
                    id: self.connection_id.clone(),
                    message: SubstreamMessage::new_close(self.substream_id.clone()),
                }),
            })
            .map_err(|e| {
                IoError::new(
                    ErrorKind::Other,
                    format!("poll_close outbound_rx error: {}", e),
                )
            })?;

        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod test {
    use super::super::message::{
        ConnectionId, Message, SubstreamId, SubstreamMessage, TransportMessage,
    };
    use super::super::mixnet::initialize_mixnet;
    use super::Substream;
    use futures::{AsyncReadExt, AsyncWriteExt};
    use nym_sdk::mixnet::MixnetClient;
    use nym_sphinx::addressing::clients::Recipient;
    use std::sync::atomic::AtomicU64;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_substream_poll_read_unread_data() {
        let (outbound_tx, _) = tokio::sync::mpsc::unbounded_channel();
        let connection_id = ConnectionId::generate();
        let substream_id = SubstreamId::generate();

        let (inbound_tx, inbound_rx) = tokio::sync::mpsc::unbounded_channel();
        let (_, close_rx) = tokio::sync::oneshot::channel();

        let mut substream = Substream::new(
            Recipient::try_from_base58_string("D1rrpsysCGCYXy9saP8y3kmNpGtJZUXN9SvFoUcqAsM9.9Ssso1ea5NfkbMASdiseDSjTN1fSWda5SgEVjdSN4CvV@GJqd3ZxpXWSNxTfx7B1pPtswpetH4LnJdFeLeuY5KUuN").unwrap(),
            connection_id,
            substream_id,
            inbound_rx,
            outbound_tx,
            close_rx,
            Arc::new(AtomicU64::new(1)),
        );

        // test writing and reading w/ same length data
        let data = b"hello".to_vec();
        inbound_tx.send(data.clone()).unwrap();
        let mut buf = [0u8; 5];
        let read_len = substream.read(&mut buf).await.unwrap();
        assert_eq!(read_len, data.len());
        assert_eq!(buf.to_vec(), data);

        // test writing data longer than read buffer
        let data = b"nootwashere".to_vec();
        inbound_tx.send(data.clone()).unwrap();

        let mut buf = [0u8; 4];
        let read_len = substream.read(&mut buf).await.unwrap();
        assert_eq!(read_len, buf.len());
        assert_eq!(buf.to_vec(), b"noot".to_vec());

        let mut buf = [0u8; 7];
        let read_len = substream.read(&mut buf).await.unwrap();
        assert_eq!(read_len, buf.len());
        assert_eq!(buf.to_vec(), b"washere".to_vec());

        // test read buffer larger than written data
        let data = b"nootwashere".to_vec();
        inbound_tx.send(data.clone()).unwrap();
        let mut buf = [0u8; 16];
        let read_len = substream.read(&mut buf).await.unwrap();
        assert_eq!(read_len, data.len());
        assert_eq!(buf[..data.len()], data);
        assert_eq!(buf[data.len()..].to_vec(), vec![0u8; 16 - data.len()]);

        // test writing data longer than read buffer multiple times
        let data = b"nootwashere".to_vec();
        inbound_tx.send(data.clone()).unwrap();

        let mut buf = [0u8; 4];
        let read_len = substream.read(&mut buf).await.unwrap();
        assert_eq!(read_len, buf.len());
        assert_eq!(buf.to_vec(), b"noot".to_vec());

        let data = b"asdf".to_vec();
        inbound_tx.send(data.clone()).unwrap();

        let mut buf = [0u8; 4];
        let read_len = substream.read(&mut buf).await.unwrap();
        assert_eq!(read_len, buf.len());
        assert_eq!(buf.to_vec(), b"wash".to_vec());

        let mut buf = [0u8; 8];
        let read_len = substream.read(&mut buf).await.unwrap();
        assert_eq!(read_len, 7);
        assert_eq!(buf[..7], b"ereasdf".to_vec());
    }

    #[tokio::test]
    async fn test_substream_read_write() {
        let client = MixnetClient::connect_new().await.unwrap();
        let (self_address, mut mixnet_inbound_rx, outbound_tx) =
            initialize_mixnet(client, None).await.unwrap();

        const MSG_INNER: &[u8] = "hello".as_bytes();
        let connection_id = ConnectionId::generate();
        let substream_id = SubstreamId::generate();

        let (inbound_tx, inbound_rx) = tokio::sync::mpsc::unbounded_channel();
        let (_, close_rx) = tokio::sync::oneshot::channel();

        let mut substream = Substream::new(
            self_address,
            connection_id,
            substream_id,
            inbound_rx,
            outbound_tx,
            close_rx,
            Arc::new(AtomicU64::new(1)),
        );

        // send message to ourselves over the mixnet
        substream.write_all(MSG_INNER).await.unwrap();

        // receive full message over the mixnet
        let recv_msg = mixnet_inbound_rx.recv().await.unwrap();
        match recv_msg.0 {
            Message::TransportMessage(TransportMessage {
                nonce,
                id: _,
                message:
                    SubstreamMessage {
                        substream_id: _,
                        message_type: msg,
                    },
            }) => {
                assert_eq!(nonce, 1);
                match msg {
                    super::super::message::SubstreamMessageType::Data(data) => {
                        assert_eq!(data, MSG_INNER);
                        // send message to substream inbound channel
                        inbound_tx.send(data).unwrap();
                    }
                    _ => panic!("unexpected message type"),
                }
            }
            _ => panic!("unexpected message"),
        }

        // read message from substream
        let mut buf = [0u8; MSG_INNER.len()];
        substream.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf, MSG_INNER);

        // close substream
        substream.close().await.unwrap();

        // try to read/write to closed substream; should error
        substream.write_all(MSG_INNER).await.unwrap_err();
        substream.read_exact(&mut buf).await.unwrap_err();

        // assert a close message was sent over the mixnet
        let recv_msg = mixnet_inbound_rx.recv().await.unwrap();
        match recv_msg.0 {
            Message::TransportMessage(TransportMessage {
                nonce: _,
                id: _,
                message:
                    SubstreamMessage {
                        substream_id: _,
                        message_type: msg,
                    },
            }) => match msg {
                super::super::message::SubstreamMessageType::Close => {}
                _ => panic!("unexpected message type"),
            },
            _ => panic!("unexpected message: {:?}", recv_msg.0),
        }
    }

    #[tokio::test]
    async fn test_substream_recv_close() {
        let client = MixnetClient::connect_new().await.unwrap();
        let (self_address, _, outbound_tx) = initialize_mixnet(client, None).await.unwrap();

        const MSG_INNER: &[u8] = "hello".as_bytes();
        let connection_id = ConnectionId::generate();
        let substream_id = SubstreamId::generate();

        let (_, inbound_rx) = tokio::sync::mpsc::unbounded_channel();
        let (close_tx, close_rx) = tokio::sync::oneshot::channel();

        let mut substream = Substream::new(
            self_address,
            connection_id,
            substream_id,
            inbound_rx,
            outbound_tx,
            close_rx,
            Arc::new(AtomicU64::new(1)),
        );

        // close substream
        close_tx.send(()).unwrap();

        // try to read/write to closed substream; should error
        substream.write_all(MSG_INNER).await.unwrap_err();
        let mut buf = [0u8; MSG_INNER.len()];
        substream.read_exact(&mut buf).await.unwrap_err();
    }
}
