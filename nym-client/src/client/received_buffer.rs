use futures::channel::{mpsc, oneshot};
use futures::lock::Mutex as FMutex;
use futures::StreamExt;
use log::{error, info, trace};
use std::sync::Arc;

pub type BufferResponse = oneshot::Sender<Vec<Vec<u8>>>;

pub(crate) struct ReceivedMessagesBuffer {
    inner: Arc<FMutex<Inner>>,
}

impl ReceivedMessagesBuffer {
    pub(crate) fn new() -> Self {
        ReceivedMessagesBuffer {
            inner: Arc::new(FMutex::new(Inner::new())),
        }
    }

    pub(crate) async fn start_controllers(
        self,
        poller_rx: mpsc::UnboundedReceiver<Vec<Vec<u8>>>, // to receive new messages
        query_receiver: mpsc::UnboundedReceiver<BufferResponse>, // to receive requests to acquire all stored messages
    ) {
        let input_controller_future = tokio::spawn(Self::run_poller_input_controller(
            self.inner.clone(),
            poller_rx,
        ));
        let output_controller_future = tokio::spawn(Self::run_query_output_controller(
            self.inner,
            query_receiver,
        ));

        futures::future::select(input_controller_future, output_controller_future).await;
        panic!("One of the received buffer controllers failed!")
    }

    pub(crate) async fn run_poller_input_controller(
        buf: Arc<FMutex<Inner>>,
        mut poller_rx: mpsc::UnboundedReceiver<Vec<Vec<u8>>>,
    ) {
        info!("Started Received Messages Buffer Input Controller");

        while let Some(new_messages) = poller_rx.next().await {
            Inner::add_new_messages(&*buf, new_messages).await;
        }
    }

    pub(crate) async fn run_query_output_controller(
        buf: Arc<FMutex<Inner>>,
        mut query_receiver: mpsc::UnboundedReceiver<BufferResponse>,
    ) {
        info!("Started Received Messages Buffer Output Controller");

        while let Some(request) = query_receiver.next().await {
            let messages = Inner::acquire_and_empty(&*buf).await;
            if let Err(failed_messages) = request.send(messages) {
                error!(
                    "Failed to send the messages to the requester. Adding them back to the buffer"
                );
                Inner::add_new_messages(&*buf, failed_messages).await;
            }
        }
    }
}

pub(crate) struct Inner {
    messages: Vec<Vec<u8>>,
}

impl Inner {
    fn new() -> Self {
        Inner {
            messages: Vec::new(),
        }
    }

    async fn add_new_messages(buf: &FMutex<Self>, msgs: Vec<Vec<u8>>) {
        trace!("Adding new messages to the buffer! {:?}", msgs);
        let mut unlocked = buf.lock().await;
        unlocked.messages.extend(msgs);
    }

    async fn acquire_and_empty(buf: &FMutex<Self>) -> Vec<Vec<u8>> {
        trace!("Emptying the buffer and returning all messages");
        let mut unlocked = buf.lock().await;
        std::mem::replace(&mut unlocked.messages, Vec::new())
    }
}
