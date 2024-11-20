// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::VecDeque;

use crate::clients::{
    connection::ConnectionStats, gateway_conn_statistics::GatewayStats,
    nym_api_statistics::NymApiStats, packet_statistics::PacketStatistics,
};

use super::error::StatsError;

use nym_task::TaskClient;
use serde::{Deserialize, Serialize};
use sysinfo::System;
use time::OffsetDateTime;
use log::warn;

const KIND: &str = "client_stats_report";
const VERSION: &str = "v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OsInformation {
    pub(crate) os_type: String,
    pub(crate) os_version: Option<String>,
    pub(crate) os_arch: Option<String>,
}

impl OsInformation {
    pub fn new() -> Self {
        OsInformation {
            os_type: System::distribution_id(),
            os_version: System::long_os_version(),
            os_arch: System::cpu_arch(),
        }
    }
}

impl Default for OsInformation {
    fn default() -> Self {
        Self::new()
    }
}

/// generic channel for sending serialized statistics data.
pub type DataSender = tokio::sync::mpsc::UnboundedSender<Vec<u8>>;

/// generic channel for sending serialized statistics data.
pub type DataReceiver = tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>;

/// Various outgoing channels used for reporting metrics / statistics data.
pub enum Sink {
    Logging,
    Chan(DataSender),
    BufferedChan(BufferedDataSender),
    // Reports to the local client to apply / display stats locally e.g using a GUI client
    TaskStatus(TaskClient),
}

impl Sink {
    pub(crate) async fn report(&mut self, data: &impl AsRef<str>) {
        match self {
            Self::Chan(ch) => {
                ch.send(data.as_ref().as_bytes().to_vec())
                    .map_err(log_stats_send_err);
            }
            Self::BufferedChan(ref mut ch) => ch.report(data).await,
            _ => {}
        }
    }

    pub(crate) async fn local_report(&mut self, data: &impl AsRef<str>) {
        match self {
            Self::Logging => {log::info!("{}", data.as_ref())}
            Self::TaskStatus(task_client) => {
                todo!("");


        
                // if let Some(rates) = rates {
                //     task_client.send_status_msg(Box::new(data.as_ref().to_string()));
                // }
            }
            _ => {}
        }
    }
}

pub struct Sinks  (Vec<Sink>);

impl Sinks {
    pub(crate) async fn report(&mut self, data: &impl AsRef<str>) {
        for s in self.0.iter_mut() {
            s.report(&data).await
        }
    }

    pub(crate) async fn local_report(&mut self, data: &impl AsRef<str>) {
        for s in self.0.iter_mut() {
            s.local_report(&data).await
        }
    }

}

/// Simple buffered data sender.
/// 
/// Allows stats messages to be written into a channel when the channel is available. When the channel is not available,
/// messages are buffered until either this object is dropped or a channel becomes available during a reporting period. 
/// 
/// This is useful for collecting anonymous startup metrics before a mixnet connection exists, while still waiting for a
/// mixnet connection to be established in order to report.
pub struct BufferedDataSender {
    messages: VecDeque<Vec<u8>>,
    sink: Option<DataSender>,
}

impl Default for BufferedDataSender {
    fn default() -> Self {
        BufferedDataSender {
            messages: VecDeque::new(),
            sink: None,
        }
    }
}

impl BufferedDataSender {
    pub fn set_sender(&mut self, sink: DataSender) {
        self.sink = Some(sink);
    } 

    async fn report(&mut self, data: &impl AsRef<str>) {
        match &self.sink {
            Some(ch) => {
                // start by sending all buffered messages
                while let Some(msg) = self.messages.pop_front() {
                    ch.send(msg)
                        .map_err(log_stats_send_err);

                }
                // send new message
                ch.send(data.as_ref().as_bytes().to_vec())
                    .map_err(log_stats_send_err);
            }
            // enqueue the message to be sent when a channel becomes available
            None => self.messages.push_back(data.as_ref().as_bytes().to_vec()),

        }        
    }
}

fn log_stats_send_err(e: impl std::error::Error) {
    warn!("failed to send stats message: {e:?}");
}

/// Report object containing both data to be reported and client / device context. We take extra care not to
/// over-capture context information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientStatsReport {
    pub(crate) kind: String,
    pub(crate) api_version: String,
    pub(crate) last_update_time: OffsetDateTime,
    pub(crate) client_id: String,
    pub(crate) client_type: String,
    pub(crate) os_information: OsInformation,
    pub(crate) packet_stats: PacketStatistics,
    pub(crate) gateway_conn_stats: GatewayStats,
    pub(crate) nym_api_stats: NymApiStats,
    pub(crate) connection_stats: ConnectionStats,
}

impl From<ClientStatsReport> for Vec<u8> {
    fn from(value: ClientStatsReport) -> Self {
        // safety, no custom serialization
        #[allow(clippy::unwrap_used)]
        let report_json = serde_json::to_string(&value).unwrap();
        report_json.as_bytes().to_vec()
    }
}

impl TryFrom<&[u8]> for ClientStatsReport {
    type Error = StatsError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(serde_json::from_slice(value)?)
    }
}

impl Default for ClientStatsReport {
    fn default() -> Self {
        ClientStatsReport {
            kind: KIND.to_string(),
            api_version: VERSION.to_string(),
            last_update_time: OffsetDateTime::now_utc(),
            client_id: Default::default(),
            client_type: Default::default(),
            os_information: Default::default(),
            packet_stats: Default::default(),
            gateway_conn_stats: Default::default(),
            nym_api_stats: Default::default(),
            connection_stats: Default::default(),
        }
    }
}

impl ToString for ClientStatsReport {
    fn to_string(&self) -> String {
        // safety, no custom serialization
        #[allow(clippy::unwrap_used)]
        serde_json::to_string(self).unwrap()
    }
}

/// Report object containing data to be reported internally to the client.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LocalStatsReport {
    pub(crate) packet_stats: PacketStatistics,
    pub(crate) gateway_conn_stats: GatewayStats,
    pub(crate) nym_api_stats: NymApiStats,
    pub(crate) connection_stats: ConnectionStats,
}

impl From<LocalStatsReport> for Vec<u8> {
    fn from(value: LocalStatsReport) -> Self {
        // safety, no custom serialization
        #[allow(clippy::unwrap_used)]
        let report_json = serde_json::to_string(&value).unwrap();
        report_json.as_bytes().to_vec()
    }
}

impl TryFrom<&[u8]> for LocalStatsReport {
    type Error = StatsError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(serde_json::from_slice(value)?)
    }
}

impl ToString for LocalStatsReport {
    fn to_string(&self) -> String {
        // safety, no custom serialization
        #[allow(clippy::unwrap_used)]
        serde_json::to_string(self).unwrap()
    }
}