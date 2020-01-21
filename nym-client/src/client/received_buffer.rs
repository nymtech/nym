use futures::channel::{mpsc, oneshot};
use futures::lock::Mutex as FMutex;
use futures::StreamExt;
use log::{error, info, trace};
use std::sync::Arc;

pub type BufferResponse = oneshot::Sender<Vec<Vec<u8>>>;

pub(crate) struct ReceivedMessagesBuffer {
    messages: Vec<Vec<u8>>,
}

impl ReceivedMessagesBuffer {
    pub(crate) fn new() -> Self {
        ReceivedMessagesBuffer {
            messages: Vec::new(),
        }
    }

    pub(crate) fn add_arc_futures_mutex(self) -> Arc<FMutex<Self>> {
        Arc::new(FMutex::new(self))
    }

    pub(crate) async fn add_new_messages(buf: Arc<FMutex<Self>>, msgs: Vec<Vec<u8>>) {
        trace!("Adding new messages to the buffer! {:?}", msgs);
        let mut unlocked = buf.lock().await;
        unlocked.messages.extend(msgs);
    }

    pub(crate) async fn run_poller_input_controller(
        buf: Arc<FMutex<Self>>,
        mut poller_rx: mpsc::UnboundedReceiver<Vec<Vec<u8>>>,
    ) {
        info!("Started Received Messages Buffer Input Controller");
        while let Some(new_messages) = poller_rx.next().await {
            ReceivedMessagesBuffer::add_new_messages(buf.clone(), new_messages).await;
        }
    }

    pub(crate) async fn acquire_and_empty(buf: Arc<FMutex<Self>>) -> Vec<Vec<u8>> {
        trace!("Emptying the buffer and returning all messages");
        let mut unlocked = buf.lock().await;
        std::mem::replace(&mut unlocked.messages, Vec::new())
    }

    pub(crate) async fn run_query_output_controller(
        buf: Arc<FMutex<Self>>,
        mut query_receiver: mpsc::UnboundedReceiver<BufferResponse>,
    ) {
        info!("Started Received Messages Buffer Output Controller");

        while let Some(request) = query_receiver.next().await {
            let messages = ReceivedMessagesBuffer::acquire_and_empty(buf.clone()).await;
            if let Err(failed_messages) = request.send(messages) {
                error!(
                    "Failed to send the messages to the requester. Adding them back to the buffer"
                );
                ReceivedMessagesBuffer::add_new_messages(buf.clone(), failed_messages).await;
            }
        }
    }
}
