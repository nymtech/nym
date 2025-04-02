// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::throughput_tester::stats::ClientStats;
use colored::Colorize;
use human_repr::{HumanCount, HumanDuration, HumanThroughput};
use nym_task::ShutdownToken;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::time::Duration;
use sysinfo::System;
use time::OffsetDateTime;
use tokio::select;
use tokio::time::{interval, Instant};
use tracing::{error, info, Span};
use tracing_indicatif::span_ext::IndicatifSpanExt;

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct StatsRecord {
    timestamp: i64,
    receive_rate: f64,
    latency: u64,
    sent: usize,
    received: usize,
}

pub(crate) struct GlobalStatsUpdater {
    system: System,
    last_update: Instant,
    last_total_received: usize,
    header_span: Span,
    client_stats: Vec<ClientStats>,

    global_records: Vec<StatsRecord>,
    records: HashMap<usize, Vec<StatsRecord>>,

    output_directory: PathBuf,
    shutdown: ShutdownToken,
}

impl GlobalStatsUpdater {
    pub(crate) fn new(
        header_span: Span,
        client_stats: Vec<ClientStats>,
        output_directory: PathBuf,
        shutdown: ShutdownToken,
    ) -> Self {
        let mut system_info = System::new_all();
        system_info.refresh_cpu_usage();

        // pre-allocate vecs
        let mut records = HashMap::new();
        for (i, _) in client_stats.iter().enumerate() {
            records.insert(i, vec![]);
        }

        GlobalStatsUpdater {
            system: system_info,
            last_update: Instant::now(),
            last_total_received: 0,
            header_span,
            client_stats,
            global_records: vec![],
            records,
            output_directory,
            shutdown,
        }
    }

    fn update_stats_span(&mut self) {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let time_delta_secs = self.last_update.elapsed().as_secs_f64();

        let mut all_received = 0;
        let mut all_sent = 0;
        let mut all_latencies = 0;
        for (i, stat) in self.client_stats.iter().enumerate() {
            // SAFETY: we create all entries during initialisation
            #[allow(clippy::unwrap_used)]
            let records = self.records.get_mut(&i).unwrap();

            let mut client_record = StatsRecord::default();

            let sent = stat.sent();
            let received = stat.received();
            let latency = stat.average_latency_nanos();

            client_record.timestamp = now;
            client_record.received = received;
            client_record.sent = sent;
            client_record.latency = latency;
            if let Some(last) = records.last() {
                let receive_rate = (received - last.received) as f64 / time_delta_secs;
                client_record.receive_rate = receive_rate;
            }
            records.push(client_record);

            all_sent += sent;
            all_received += received;
            all_latencies += latency;
        }

        let receive_rate = (all_received - self.last_total_received) as f64 / time_delta_secs;
        let avg_rate = receive_rate.human_throughput("packets");
        let avg_latency = all_latencies as f64 / self.client_stats.len() as f64;

        self.global_records.push(StatsRecord {
            timestamp: now,
            receive_rate,
            latency: avg_latency as u64,
            sent: all_sent,
            received: all_received,
        });

        self.system.refresh_cpu_usage();
        let cpu_usage = self.system.global_cpu_usage();
        let cpu_count = self.system.cpus().len();
        let usage_per_cpu = cpu_usage / cpu_count as f32;

        let formatted_usage = if usage_per_cpu < 0.3 {
            format!("{:.2}%", usage_per_cpu * 100.).green().bold()
        } else if usage_per_cpu < 0.7 {
            format!("{:.2}%", usage_per_cpu * 100.).yellow().bold()
        } else {
            format!("{:.2}%", usage_per_cpu * 100.).red().bold()
        };

        self.header_span.pb_set_message(&format!(
            "active_clients: {} | total received: {} total sent {} (avg packet latency: {}, total receive rate: {avg_rate}), avg core load: {formatted_usage}",
            self.client_stats.len(),
            all_received.human_count_bare(),
            all_sent.human_count_bare(),
            Duration::from_nanos(avg_latency as u64).human_duration()
        ));
        self.last_total_received = all_received;
        self.last_update = Instant::now();
    }

    fn save_results_to_files(&self) -> anyhow::Result<()> {
        create_dir_all(self.output_directory.as_path())?;

        let global = self.output_directory.join("global_stats.csv");
        let mut writer = csv::Writer::from_path(&global)?;
        for record in &self.global_records {
            writer.serialize(record)?;
        }

        info!("wrote global stats to {}", global.display());

        for (sender_id, records) in self.records.iter() {
            let output = self
                .output_directory
                .join(format!("sender{}.csv", sender_id));
            let mut writer = csv::Writer::from_path(&output)?;
            for record in records {
                writer.serialize(record)?;
            }
            info!("wrote client records to {}", output.display());
        }
        Ok(())
    }

    pub(crate) async fn run(&mut self) {
        let mut update_interval = interval(Duration::from_millis(500));

        loop {
            select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    if let Err(err) = self.save_results_to_files() {
                        error!("failed to save measurement results to files: {err}")
                    }
                    break;
                }
                _ = update_interval.tick() => {
                    self.update_stats_span();
                }
            }
        }
    }
}
