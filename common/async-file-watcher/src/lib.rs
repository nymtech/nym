// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use futures::StreamExt;
use notify::event::{DataChange, MetadataKind, ModifyKind};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::Instant;

pub use notify::{Error as NotifyError, Result as NotifyResult};

pub type FileWatcherEventSender = mpsc::UnboundedSender<Event>;
pub type FileWatcherEventReceiver = mpsc::UnboundedReceiver<Event>;

/// Simple file watcher that sends a notification whenever there was any changed in the watched file.
pub struct AsyncFileWatcher {
    path: PathBuf,
    watcher: RecommendedWatcher,
    is_watching: bool,
    filters: Option<Vec<EventKind>>,
    last_received: HashMap<EventKind, Instant>,
    tick_duration: Duration,

    inner_rx: mpsc::UnboundedReceiver<NotifyResult<Event>>,
    event_sender: FileWatcherEventSender,
}

impl AsyncFileWatcher {
    pub fn new_file_changes_watcher<P: AsRef<Path>>(
        path: P,
        event_sender: FileWatcherEventSender,
    ) -> NotifyResult<Self> {
        Self::new(
            path,
            event_sender,
            Some(vec![
                EventKind::Modify(ModifyKind::Data(DataChange::Content)),
                EventKind::Modify(ModifyKind::Data(DataChange::Any)),
                EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any)),
            ]),
            None,
        )
    }

    pub fn new<P: AsRef<Path>>(
        path: P,
        event_sender: FileWatcherEventSender,
        filters: Option<Vec<EventKind>>,
        tick_duration: Option<Duration>,
    ) -> NotifyResult<Self> {
        let watcher_config = Config::default();
        let (inner_tx, inner_rx) = mpsc::unbounded();
        let watcher = RecommendedWatcher::new(
            move |res| {
                if let Err(_err) = inner_tx.unbounded_send(res) {
                    // I guess it's theoretically possible during shutdown?
                    log::error!(
                        "failed to send watched file event - the received must have been dropped!"
                    );
                }
            },
            watcher_config,
        )?;

        Ok(AsyncFileWatcher {
            path: path.as_ref().to_path_buf(),
            watcher,
            is_watching: false,
            filters,
            last_received: HashMap::new(),
            tick_duration: tick_duration.unwrap_or(Duration::from_secs(5)),
            inner_rx,
            event_sender,
        })
    }

    pub fn with_filters(mut self, filters: Option<Vec<EventKind>>) -> Self {
        self.filters = filters;
        self
    }

    pub fn with_filter(mut self, filter: EventKind) -> Self {
        match &mut self.filters {
            None => {
                self.filters = Some(vec![filter]);
            }
            Some(filters) => filters.push(filter),
        }
        self
    }

    fn should_propagate(&self, event: &Event, now: Instant) -> bool {
        // when testing I was consistently getting two `Modify(Data(Any))` events in quick succession
        // (probably to modify content and metadata).
        // we really only want to propagate one of them
        if let Some(previous) = self.last_received.get(&event.kind) {
            if now.duration_since(*previous) < self.tick_duration {
                return false;
            }
        }

        let Some(filters) = &self.filters else {
            return true;
        };

        for filter in filters {
            if &event.kind == filter {
                return true;
            }
        }
        false
    }

    fn start_watching(&mut self) -> NotifyResult<()> {
        self.is_watching = true;
        self.watcher.watch(&self.path, RecursiveMode::NonRecursive)
    }

    fn stop_watching(&mut self) -> NotifyResult<()> {
        self.is_watching = false;
        self.watcher.unwatch(&self.path)
    }

    pub async fn watch(&mut self) -> NotifyResult<()> {
        self.start_watching()?;

        while let Some(event) = self.inner_rx.next().await {
            match event {
                Ok(event) => {
                    let now = Instant::now();
                    if self.should_propagate(&event, now) {
                        self.last_received.insert(event.kind, now);
                        if let Err(_err) = self.event_sender.unbounded_send(event) {
                            log::error!("the file watcher receiver has been dropped!");
                        }
                    } else {
                        log::debug!("will not propagate information about {:?}", event);
                    }
                }
                Err(err) => {
                    // TODO: to be determined if this should stop the whole thing or not
                    // (need to know what kind of errors can be returned)
                    log::error!(
                        "encountered an error while watching {:?}: {err}",
                        self.path.as_path()
                    );
                }
            }
        }

        self.stop_watching()
    }

    pub fn is_watching(&self) -> bool {
        self.is_watching
    }
}
