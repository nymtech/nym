use nym_task::ShutdownTracker;

pub(crate) type EventReceiver = tokio::sync::mpsc::UnboundedReceiver<()>;
pub(crate) type EventSender = tokio::sync::mpsc::UnboundedSender<()>;

pub(crate) struct ShutdownControl {
    external_shutdown_tracker: ShutdownTracker,
    internal_shutdown_tracker: ShutdownTracker,
    event_rx: EventReceiver,
}

impl ShutdownControl {
    pub(crate) fn new(
        external_shutdown_tracker: ShutdownTracker,
        internal_shutdown_tracker: ShutdownTracker,
        event_rx: EventReceiver,
    ) -> Self {
        Self {
            external_shutdown_tracker,
            internal_shutdown_tracker,
            event_rx,
        }
    }

    pub(crate) async fn run(self) {
        let external_shutdown_token = self.external_shutdown_tracker.clone_shutdown_token();
        loop {
            tokio::select! {
                biased;
                _ = external_shutdown_token.cancelled() => {
                    self
                    tracing::trace!("OutQueueControl: Received shutdown");
                    break;
                }
                _ = status_timer.tick() => {
                    self.log_status();
                }
                next_message = self.next() => if let Some(next_message) = next_message {
                    self.on_message(next_message).await;
                } else {
                    tracing::trace!("OutQueueControl: Stopping since channel closed");
                    break;
                }
            }
        }
    }
}
