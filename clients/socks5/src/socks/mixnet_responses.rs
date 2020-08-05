use super::client::{ActiveStreams, RequestID};
use crate::client::received_buffer::ReconstructedMessagesReceiver;
use crate::client::received_buffer::{ReceivedBufferMessage, ReceivedBufferRequestSender};
use futures::channel::mpsc;
use futures::StreamExt;
use log::*;

#[derive(Debug)]
pub(crate) enum MixnetResponseError {
    InvalidResponseError,
}

pub(crate) struct MixnetResponseListener {
    buffer_requester: ReceivedBufferRequestSender,
    mix_response_receiver: ReconstructedMessagesReceiver,
    active_streams: ActiveStreams,
}

impl Drop for MixnetResponseListener {
    fn drop(&mut self) {
        self.buffer_requester
            .unbounded_send(ReceivedBufferMessage::ReceiverDisconnect)
            .expect("the buffer request failed!")
    }
}

impl MixnetResponseListener {
    pub(crate) fn new(
        buffer_requester: ReceivedBufferRequestSender,
        active_streams: ActiveStreams,
    ) -> Self {
        let (mix_response_sender, mix_response_receiver) = mpsc::unbounded();
        buffer_requester
            .unbounded_send(ReceivedBufferMessage::ReceiverAnnounce(mix_response_sender))
            .unwrap();

        MixnetResponseListener {
            active_streams,
            buffer_requester,
            mix_response_receiver,
        }
    }

    fn parse_message(&self, message: Vec<u8>) -> Result<(RequestID, Vec<u8>), MixnetResponseError> {
        if message.len() < 8 {
            return Err(MixnetResponseError::InvalidResponseError);
        }

        let mut request_id_bytes = message;
        let response = request_id_bytes.split_off(8);

        let request_id = u64::from_be_bytes([
            request_id_bytes[0],
            request_id_bytes[1],
            request_id_bytes[2],
            request_id_bytes[3],
            request_id_bytes[4],
            request_id_bytes[5],
            request_id_bytes[6],
            request_id_bytes[7],
        ]);

        Ok((request_id, response))
    }

    async fn on_message(&self, message: Vec<u8>) {
        let (request_id, response) = match self.parse_message(message) {
            Err(err) => {
                warn!("failed to parse received response - {:?}", err);
                return;
            }
            Ok(data) => data,
        };

        let mut active_streams_guard = self.active_streams.lock().await;
        // `remove` gives back the entry (assuming it exists). There's no reason for it to persist
        // after we send data back
        if let Some(stream_receiver) = active_streams_guard.remove(&request_id) {
            stream_receiver.send(response).unwrap()
        } else {
            warn!("no request_id exists with id: {:?}", request_id)
        }
    }

    pub(crate) async fn run(&mut self) {
        while let Some(received_responses) = self.mix_response_receiver.next().await {
            println!(
                "\n\nRECEIVED MIXNET MESSAGES!! - {:?}\n\n",
                received_responses
            );
            for received_response in received_responses {
                self.on_message(received_response).await;
            }
        }
        println!("We should never see this message");
    }
}
