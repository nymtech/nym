use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use std::io;
use std::net::SocketAddr;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

type ConnectionErrorResponse = (SocketAddr, io::Result<()>);
type ConnectionErrorSender = mpsc::UnboundedSender<ConnectionErrorResponse>;
type ConnectionErrorReceiver = mpsc::UnboundedReceiver<ConnectionErrorResponse>;

struct ConnectionErrorReader {
    error_rx: ConnectionErrorReceiver,
}

impl ConnectionErrorReader {
    fn new(error_rx: ConnectionErrorReceiver) -> Self {
        ConnectionErrorReader { error_rx }
    }

    fn start(mut self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
            while let Some(err_res) = self.error_rx.next().await {
                let (source, err) = err_res;
                match err {
                    Ok(_) => trace!("packet to {} was sent successfully!", source.to_string()),
                    Err(e) => warn!("failed to send packet to {} - {:?}", source.to_string(), e),
                }
            }
        })
    }
}
