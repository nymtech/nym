// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{any::Any, fmt};

pub type SentStatus = Box<dyn TaskStatusEvent>;
pub type StatusSender = futures::channel::mpsc::Sender<SentStatus>;
pub type StatusReceiver = futures::channel::mpsc::Receiver<SentStatus>;

pub trait TaskStatusEvent: Send + Sync + Any + fmt::Display {
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Ready,
    ReadyWithGateway(String),
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Ready => write!(f, "Ready"),
            TaskStatus::ReadyWithGateway(gateway) => {
                write!(f, "Ready and connected to gateway: {gateway}")
            }
        }
    }
}

impl TaskStatusEvent for TaskStatus {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
