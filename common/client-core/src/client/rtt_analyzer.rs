use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};

use once_cell::sync::OnceCell;
use std::process::Command;
use std::sync::Mutex;
use tokio::sync::mpsc::{self, Sender};
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub enum RttPattern {
    Burst,
    RoundRobin,
}

#[derive(Debug, Clone)]
pub struct RttConfig {
    pub packets_per_route: u32,
    pub pattern: RttPattern,
    pub inter_route_delay_ms: u64,
}

#[derive(Debug, Clone)]
pub enum RttEvent {
    RouteUsed {
        route_index: usize,
        fragment_id: String,
    },
    FragmentSent {
        fragment_id: String,
        timestamp: u128,
    },
    FragmentAckReceived {
        fragment_id: String,
        timestamp: u128,
    },
    FragmentAckExpired {
        fragment_id: String,
        timestamp: u128,
    },
    FragmentReceived {
        fragment_id: String,
        timestamp: u128,
    },
    RouteNodes {
        route_index: usize,
        nodes: String,
    },
    ExperimentConfiguration {
        total_routes: usize,
        per_route_sent: usize,
    },
    PrintRouteDetail {
        route_index: usize,
    },
    PrintRouteStatsByNodes {
        nodes: String,
    },
    PrintRoutesWithAvgAbove {
        threshold_ms: u128,
    },
    PrintRoutesWithAnyAbove {
        threshold_ms: u128,
    },
    PrintStats,
    WriteStats {
        path: String,
    },
    WriteStatsAndPlot {
        path: String,
        outlier_mode: String, // "all" or cutoff() seconds (e.g. "1.0"))
    },
    PrintExperimentProgress,
}

pub struct StoredRouteSummary {
    pub total_routes: usize,
    pub per_route_sent: usize,
}

static PRODUCER: OnceCell<Mutex<Option<Sender<RttEvent>>>> = OnceCell::new();

#[derive(Default, Debug)]
pub struct RouteStats {
    pub sent: u32,
    pub acks: u32,
    pub timeouts: u32,
    pub rtts: Vec<u128>,
}

pub struct RttAnalyzer {
    /// fragment_id → (route, Vec<sent_times>)
    fragments: HashMap<String, (usize, Vec<u128>)>,

    /// fragment_id → Vec<recv_times>
    receive_times: HashMap<String, Vec<u128>>,

    /// fragment_id → last ack
    ack_times: HashMap<String, u128>,

    route_stats: HashMap<usize, RouteStats>,
    route_summary: Option<StoredRouteSummary>,
    route_nodes: HashMap<usize, String>,
    consumer_handle: JoinHandle<()>,
}

impl RttAnalyzer {
    pub fn consumer_handle(&self) -> &JoinHandle<()> {
        &self.consumer_handle
    }

    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel(80000);

        PRODUCER
            .set(Mutex::new(Some(tx.clone())))
            .expect("PRODUCER already initialized");

        let handle = tokio::spawn(async move {
            let mut analyzer = RttAnalyzer {
                fragments: HashMap::new(),
                receive_times: HashMap::new(),
                ack_times: HashMap::new(),
                route_stats: HashMap::new(),
                route_summary: None,
                route_nodes: HashMap::new(),
                consumer_handle: tokio::spawn(async {}),
            };

            while let Some(event) = rx.recv().await {
                analyzer.process(event);
            }

            println!("RTT Analyzer consumer exited");
        });

        Self {
            fragments: HashMap::new(),
            receive_times: HashMap::new(),
            ack_times: HashMap::new(),
            route_stats: HashMap::new(),
            route_summary: None,
            route_nodes: HashMap::new(),
            consumer_handle: handle,
        }
    }

    pub fn producer() -> Option<Sender<RttEvent>> {
        let lock = PRODUCER.get()?.lock().unwrap();
        lock.clone()
    }

    pub fn process(&mut self, event: RttEvent) {
        match event {
            // -------------------------
            // FIRST USE OF A FRAGMENT
            // -------------------------
            RttEvent::RouteUsed {
                route_index,
                fragment_id,
            } => {
                self.fragments
                    .insert(fragment_id.clone(), (route_index, Vec::new()));
            }

            // -------------------------
            // RETRANSMISSION → append new sent time
            // -------------------------
            RttEvent::FragmentSent {
                fragment_id,
                timestamp,
            } => {
                if let Some((_route, sent_list)) = self.fragments.get_mut(&fragment_id) {
                    sent_list.push(timestamp);
                    self.route_stats.entry(*_route).or_default().sent += 1;
                }
            }

            // -------------------------
            // ACK RECEIVED
            // -------------------------
            RttEvent::FragmentAckReceived {
                fragment_id,
                timestamp,
            } => {
                if let Some((route, _)) = self.fragments.get(&fragment_id) {
                    let stats = self.route_stats.entry(*route).or_default();
                    stats.acks += 1;
                }
                self.ack_times.insert(fragment_id, timestamp);
            }

            // -------------------------
            // ACK TIMEOUT
            // -------------------------
            RttEvent::FragmentAckExpired { fragment_id, .. } => {
                if let Some((route, _)) = self.fragments.get(&fragment_id) {
                    self.route_stats.entry(*route).or_default().timeouts += 1;
                }
            }
            RttEvent::WriteStatsAndPlot { path, outlier_mode } => {
                // 1) write the csv
                if let Err(e) = self.write_csv(&path) {
                    eprintln!("Failed to write CSV: {}", e);
                    return;
                }

                // 2) Call the Python script
                if let Err(e) = Self::run_histogram_script(&path, &outlier_mode) {
                    eprintln!("Failed to run histogram script: {}", e);
                }
            }

            // -------------------------
            // PACKET RECEIVED → compute RTTs
            // -------------------------
            RttEvent::FragmentReceived {
                fragment_id,
                timestamp,
            } => {
                // Append receive time
                let recv_list = self.receive_times.entry(fragment_id.clone()).or_default();
                recv_list.push(timestamp);

                // Lookup route + sent times
                if let Some((route, sent_list)) = self.fragments.get(&fragment_id) {
                    let recv_list = self.receive_times.get(&fragment_id).unwrap();

                    // Index of the *newly added* receive time
                    let idx = recv_list.len() - 1;
                    /*
                    Maybe we can put a retransmission flag and not counting the new RTTs and only the basic N?
                     */
                    //println!("Fragment id: {} Sent list length: {} Recv list length:{}",fragment_id,sent_list.len(),recv_list.len());

                    // Check if we have a matching sent timestamp
                    if idx < sent_list.len() {
                        let sent_ts = sent_list[idx];
                        let rtt = recv_list[idx] - sent_ts;

                        self.route_stats.entry(*route).or_default().rtts.push(rtt);
                    }
                }
            }

            RttEvent::RouteNodes { route_index, nodes } => {
                self.route_nodes.insert(route_index, nodes);
            }

            RttEvent::PrintStats => self.print_stats(),

            RttEvent::WriteStats { path } => {
                if let Err(e) = self.write_csv(&path) {
                    eprintln!("Failed to write CSV: {}", e)
                }
            }

            RttEvent::ExperimentConfiguration {
                total_routes,
                per_route_sent,
            } => {
                self.route_summary = Some(StoredRouteSummary {
                    total_routes,
                    per_route_sent,
                });
            }
            RttEvent::PrintExperimentProgress => {
                self.print_experiment_progress();
            }

            RttEvent::PrintRouteDetail { route_index } => {
                self.print_route_detail(route_index);
            }

            RttEvent::PrintRoutesWithAvgAbove { threshold_ms } => {
                self.print_routes_with_avg_above(threshold_ms);
            }

            RttEvent::PrintRoutesWithAnyAbove { threshold_ms } => {
                self.print_routes_with_any_above(threshold_ms);
            }

            RttEvent::PrintRouteStatsByNodes { nodes } => {
                self.print_route_by_nodes(nodes);
            }
        }
    }

    // ---------------------- PRINT FUNCTIONS (unchanged) ----------------------
    pub fn print_stats(&self) {
        println!("\n================ Route RTT Statistics ================");
        for (route, stats) in self.route_stats.iter() {
            let avg_rtt = if !stats.rtts.is_empty() {
                stats.rtts.iter().sum::<u128>() as f64 / stats.rtts.len() as f64
            } else {
                0.0
            };

            println!(
                "Route {:5} | Sent {:4} | ACKs {:4} | Timeouts {:4} | Avg RTT {:8.2}",
                route, stats.sent, stats.acks, stats.timeouts, avg_rtt
            );
        }
        println!("======================================================\n");
    }

    pub fn write_csv(&self, path: &str) -> std::io::Result<()> {
        let mut writer = BufWriter::new(File::create(path)?);

        writer.write_all(b"route,sent,acks,timeouts,avg_rtt\n")?;

        for (route, stats) in &self.route_stats {
            let avg_rtt = if !stats.rtts.is_empty() {
                stats.rtts.iter().sum::<u128>() as f64 / stats.rtts.len() as f64
            } else {
                0.0
            };

            writer.write_all(
                format!(
                    "{},{},{},{},{:.2}\n",
                    route, stats.sent, stats.acks, stats.timeouts, avg_rtt
                )
                .as_bytes(),
            )?;
        }
        Ok(())
    }

    pub fn print_route_detail(&self, route_index: usize) {
        println!(
            "\n================ Route #{} Details ================\n",
            route_index
        );

        if let Some(nodes) = self.route_nodes.get(&route_index) {
            println!(" Route Nodes:");
            for (i, node) in nodes.split(" > ").enumerate() {
                println!("   • Node {}: {}", i + 1, node);
            }
        }

        if let Some(stats) = self.route_stats.get(&route_index) {
            println!("\n RTT Values:");
            for (i, rtt) in stats.rtts.iter().enumerate() {
                println!("   [{:3}] {} ms", i, rtt);
            }
        }

        println!("======================================================\n");
    }
    fn run_histogram_script(csv_path: &str, outlier_mode: &str) -> std::io::Result<()> {
        let status = Command::new("python")
            .arg("rtt_histogram.py") // path του script
            .arg(csv_path)
            .arg(outlier_mode)
            .status()?;

        if !status.success() {
            eprintln!("Python histogram script exited with status: {}", status);
        }

        Ok(())
    }

    pub fn print_routes_with_avg_above(&self, threshold_ms: u128) {
        println!(
            "\n======= Routes with AVG RTT > {} ms =======\n",
            threshold_ms
        );

        let mut matches: Vec<usize> = self
            .route_stats
            .iter()
            .filter_map(|(route, stats)| {
                if stats.rtts.is_empty() {
                    return None;
                }
                let avg = stats.rtts.iter().sum::<u128>() as f64 / stats.rtts.len() as f64;
                if (avg as u128) > threshold_ms {
                    Some(*route)
                } else {
                    None
                }
            })
            .collect();

        matches.sort();

        for route in matches {
            self.print_route_detail(route);
        }

        println!("====================================================\n");
    }
    /// Prints overall experiment completion percentage.
    ///
    /// It uses:
    /// - self.route_summary.total_routes
    /// - self.route_summary.per_route_sent
    /// to compute how many packets were planned in total.
    ///
    /// Then it sums, over all routes:
    /// - how many packets were actually sent (RouteStats.sent)
    /// - how many packets have a completed RTT sample (RouteStats.rtts.len())
    ///
    /// Finally it prints:
    /// - total expected packets
    /// - total sent packets and percentage
    /// - total completed RTT packets and percentage
    pub fn print_experiment_progress(&self) {
        println!("\n=========== RTT Experiment Progress ===========");

        // Check if experiment configuration is available
        let summary = match &self.route_summary {
            Some(s) => s,
            None => {
                println!("No experiment configuration stored (route_summary is None).");
                println!("You must send an ExperimentConfiguration event first.");
                println!("==============================================\n");
                return;
            }
        };

        let total_routes = summary.total_routes;
        let per_route_sent = summary.per_route_sent;

        // Total number of packets that were planned for the whole experiment
        let expected_total: usize = total_routes.saturating_mul(per_route_sent);

        if expected_total == 0 {
            println!("Experiment configuration has zero expected packets.");
            println!("==============================================\n");
            return;
        }

        // Sum how many packets were actually sent and how many have a measured RTT
        let mut sent_total: usize = 0;
        let mut received_total: usize = 0;

        for (_route_idx, stats) in &self.route_stats {
            // 'sent' counts how many times we called FragmentSent for this route
            sent_total += std::cmp::min(stats.sent, per_route_sent as u32) as usize;

            // Each RTT entry corresponds to one packet for which we have both send and receive time
            let route_recv = std::cmp::min(stats.rtts.len(), per_route_sent);
            received_total += route_recv;
        }

        let sent_pct = (sent_total as f64 / expected_total as f64) * 100.0;
        let recv_pct = (received_total as f64 / expected_total as f64) * 100.0;

        println!("Total routes configured       : {}", total_routes);
        println!("Packets per route (planned)   : {}", per_route_sent);
        println!("Total expected packets        : {}", expected_total);
        println!("---------------------------------------------");
        println!(
            "Total sent packets            : {} ({:.2}%)",
            sent_total, sent_pct
        );
        println!(
            "Total completed RTT packets   : {} ({:.2}%)",
            received_total, recv_pct
        );
        println!("==============================================\n");
    }

    pub fn print_routes_with_any_above(&self, threshold_ms: u128) {
        println!(
            "\n======= Routes with ANY RTT > {} ms =======\n",
            threshold_ms
        );

        let mut matches: Vec<usize> = self
            .route_stats
            .iter()
            .filter_map(|(route, stats)| {
                if stats.rtts.iter().any(|&x| x > threshold_ms) {
                    Some(*route)
                } else {
                    None
                }
            })
            .collect();

        matches.sort();

        for route in matches {
            self.print_route_detail(route);
        }

        println!("====================================================\n");
    }

    pub fn print_route_by_nodes(&self, nodes: String) {
        println!("\n========== Searching route by nodes ==========\n");

        let mut routes: Vec<(usize, &String)> =
            self.route_nodes.iter().map(|(k, v)| (*k, v)).collect();
        routes.sort_by_key(|(idx, _)| *idx);

        for (route, stored) in routes {
            if *stored == nodes {
                println!("Found route {}!", route);
                self.print_route_detail(route);
                return;
            }
        }

        println!("No route found with nodes: {}", nodes);
        println!("=============================================\n");
    }
}
