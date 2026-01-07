use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;

use nym_client_core::client::rtt_analyzer::{RttAnalyzer, RttConfig, RttEvent, RttPattern};
use nym_sdk::DebugConfig;
use tokio::io::{self, AsyncBufReadExt};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    // ============================================================
    // 1. Start RTT Analyzer + background worker
    // ============================================================
    let _analyzer = RttAnalyzer::new();

    let tx = RttAnalyzer::producer().expect("Analyzer was not initialized!");

    // ============================================================
    // 2. Build mixnet client
    // ============================================================
    let mut debug = DebugConfig::default();

    // Disable ALL Poisson & cover streams
    debug.traffic.disable_main_poisson_packet_distribution = true;
    debug.cover_traffic.disable_loop_cover_traffic_stream = false;

    let client = mixnet::MixnetClientBuilder::new_ephemeral()
        .debug_config(debug)
        .build()
        .unwrap();

    let client = client.connect_to_mixnet().await.unwrap();

    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // ============================================================
    // 3. Ask the user for RTT TEST configuration
    // ============================================================
    let config = ask_user_for_rtt_config().await;

    println!("\nStarting RTT test with:");
    println!("  packets_per_route = {}", config.packets_per_route);
    println!("  pattern           = {:?}", config.pattern);
    println!("  delay (ms)        = {}", config.inter_route_delay_ms);

    // START THE TEST
    let _ = client
        .send_rtt_test(*our_address, None, tx.clone(), config)
        .await
        .unwrap();

    // ============================================================
    // 4. Background listener for incoming messages
    // ============================================================
    tokio::spawn({
        let mut client = client;
        async move {
            loop {
                if client.wait_for_messages().await.is_some() {
                    //I should do something here to shutdown
                }
                sleep(Duration::from_millis(10)).await;
            }
        }
    });

    // ============================================================
    // 5. Main input loop
    // ============================================================
    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    println!("Type 'menu' to show RTT commands.");

    loop {
        if let Ok(Some(input)) = lines.next_line().await {
            let input = input.trim().to_lowercase();

            if input == "menu" {
                show_menu_and_handle_choice(&tx).await;
            }
        }

        sleep(Duration::from_millis(50)).await;
    }
}

// =====================================================================
// ASK USER FOR RTT TEST SETTINGS AT PROGRAM START
// =====================================================================
async fn ask_user_for_rtt_config() -> RttConfig {
    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    println!("\n========== RTT TEST CONFIGURATION ==========");

    // -----------------------------
    // Ask for packets per route
    // -----------------------------
    println!("Enter number of packets per route: ");
    let packets = read_u32_from_stdin(&mut lines).await;

    // -----------------------------
    // Ask for pattern: Burst / RR
    // -----------------------------
    println!("Choose pattern:");
    println!("  1) Burst");
    println!("  2) Round Robin");
    let pattern = loop {
        let input = read_string(&mut lines).await;

        match input.as_str() {
            "1" => break RttPattern::Burst,
            "2" => break RttPattern::RoundRobin,
            _ => println!("Invalid choice! Please type 1 or 2:"),
        }
    };

    // -----------------------------
    // Ask for delay between packets
    // -----------------------------
    println!("Enter delay between packets (ms): ");
    let delay = read_u64_from_stdin(&mut lines).await;

    // Build Config
    RttConfig {
        packets_per_route: packets,
        pattern,
        inter_route_delay_ms: delay,
    }
}

// =====================================================================
// Util functions for reading typed input
// =====================================================================
async fn read_string(lines: &mut tokio::io::Lines<io::BufReader<io::Stdin>>) -> String {
    loop {
        if let Ok(Some(line)) = lines.next_line().await {
            let trimmed = line.trim().to_string();
            if !trimmed.is_empty() {
                return trimmed;
            }
        }
        println!("Please type a value:");
    }
}

async fn read_u32_from_stdin(lines: &mut tokio::io::Lines<io::BufReader<io::Stdin>>) -> u32 {
    loop {
        if let Ok(Some(line)) = lines.next_line().await {
            if let Ok(num) = line.trim().parse::<u32>() {
                return num;
            }
        }
        println!("Invalid number, try again:");
    }
}

async fn read_u64_from_stdin(lines: &mut tokio::io::Lines<io::BufReader<io::Stdin>>) -> u64 {
    loop {
        if let Ok(Some(line)) = lines.next_line().await {
            if let Ok(num) = line.trim().parse::<u64>() {
                return num;
            }
        }
        println!("Invalid number, try again:");
    }
}
// =====================================================================
// MENU HANDLER (FULL VERSION WITH HELP / DOCS)
// =====================================================================
async fn show_menu_and_handle_choice(tx: &tokio::sync::mpsc::Sender<RttEvent>) {
    println!("\n======================== RTT MENU ========================");
    println!("1) Print global RTT statistics");
    println!("2) Write statistics to CSV file");
    println!("3) Print route details by ROUTE INDEX");
    println!("4) Print route details by ROUTE NODES STRING");
    println!("5) Print routes with AVG RTT above threshold");
    println!("6) Print routes with ANY RTT above threshold");
    println!("7) Help (Show all commands & how to use them)");
    println!("8) Write CSV and generate RTT histogram(s) with Python");
    println!("9) Show overall experiment completion percentage");
    println!("===========================================================");
    print!("Select option: ");

    use std::io::Write;
    std::io::stdout().flush().unwrap();

    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    let choice = input.trim();

    match choice {
        // -------------------- 1. PRINT GLOBAL STATS --------------------
        "1" => {
            let _ = tx.send(RttEvent::PrintStats).await;
        }

        // -------------------- 2. WRITE STATS ---------------------------
        "2" => {
            print!("Enter file path: ");
            std::io::stdout().flush().unwrap();

            let mut path = String::new();
            let _ = std::io::stdin().read_line(&mut path);

            let path = path.trim().to_string();
            let _ = tx.send(RttEvent::WriteStats { path }).await;
        }

        // -------------------- 3. PRINT ROUTE DETAILS -------------------
        "3" => {
            print!("Enter route index (0-based): ");
            std::io::stdout().flush().unwrap();

            let mut s = String::new();
            let _ = std::io::stdin().read_line(&mut s);

            if let Ok(index) = s.trim().parse::<usize>() {
                let _ = tx
                    .send(RttEvent::PrintRouteDetail { route_index: index })
                    .await;
            } else {
                println!("Invalid index.");
            }
        }

        // -------------------- 4. PRINT STATS BY NODE STRING -----------
        "4" => {
            println!("Enter Node String EXACTLY as stored.");
            println!("Example format:");
            println!("  <base58_node1> > <base58_node2> > <base58_node3>");
            print!("Nodes: ");
            std::io::stdout().flush().unwrap();

            let mut nodes = String::new();
            let _ = std::io::stdin().read_line(&mut nodes);

            let nodes = nodes.trim().to_string();
            let _ = tx.send(RttEvent::PrintRouteStatsByNodes { nodes }).await;
        }

        // -------------------- 5. AVG ABOVE THRESHOLD ------------------
        "5" => {
            print!("Enter threshold in ms: ");
            std::io::stdout().flush().unwrap();

            let mut s = String::new();
            let _ = std::io::stdin().read_line(&mut s);

            if let Ok(th) = s.trim().parse::<u128>() {
                let _ = tx
                    .send(RttEvent::PrintRoutesWithAvgAbove { threshold_ms: th })
                    .await;
            } else {
                println!("Invalid number.");
            }
        }

        // -------------------- 6. ANY ABOVE THRESHOLD ------------------
        "6" => {
            print!("Enter threshold in ms: ");
            std::io::stdout().flush().unwrap();

            let mut s = String::new();
            let _ = std::io::stdin().read_line(&mut s);

            if let Ok(th) = s.trim().parse::<u128>() {
                let _ = tx
                    .send(RttEvent::PrintRoutesWithAnyAbove { threshold_ms: th })
                    .await;
            } else {
                println!("Invalid number.");
            }
        }

        // -------------------- 7. HELP --------------------------------
        "7" => {
            print_help();
        }
        "8" => {
            // Ask for CSV path
            print!("Enter CSV output path (e.g. rtt_stats.csv): ");
            std::io::stdout().flush().unwrap();

            let mut path = String::new();
            let _ = std::io::stdin().read_line(&mut path);
            let path = path.trim().to_string();

            // Sub-menu for histogram mode
            println!("\nHistogram mode:");
            println!("  1) One plot with ALL RTT samples (including outliers)");
            println!("  2) One plot with INLIERS only (RTT <= cutoff)");
            println!("  3) TWO plots: one for INLIERS and one for OUTLIERS");
            print!("Select mode: ");
            std::io::stdout().flush().unwrap();

            let mut mode_input = String::new();
            let _ = std::io::stdin().read_line(&mut mode_input);
            let mode_choice = mode_input.trim();

            let outlier_mode = match mode_choice {
                // 1) All RTTs
                "1" => "all".to_string(),

                // 2) Only inliers, ask for cutoff in seconds
                "2" => {
                    print!("Enter cutoff in seconds (e.g. 1.0 for 1 second): ");
                    std::io::stdout().flush().unwrap();

                    let mut c = String::new();
                    let _ = std::io::stdin().read_line(&mut c);
                    let cutoff = c.trim();
                    cutoff.to_string() // e.g. "1.0"
                }

                // 3) Two plots: inliers + outliers (both)
                "3" => {
                    print!("Enter cutoff in seconds (e.g. 1.0 for 1 second): ");
                    std::io::stdout().flush().unwrap();

                    let mut c = String::new();
                    let _ = std::io::stdin().read_line(&mut c);
                    let cutoff = c.trim();
                    // Encode as 'both:<cutoff>' so Python can understand it
                    format!("both:{cutoff}")
                }

                _ => {
                    println!("Invalid mode, aborting histogram generation.");
                    return;
                }
            };

            let _ = tx
                .send(RttEvent::WriteStatsAndPlot { path, outlier_mode })
                .await;
        }

        "9" => {
            // Send an event to the RTT analyzer to compute and print progress
            let _ = tx.send(RttEvent::PrintExperimentProgress).await;
        }

        _ => println!("Invalid selection."),
    }
}

fn print_help() {
    println!("\n======================== RTT HELP ========================\n");

    println!("This tool allows you to perform detailed RTT analysis over all mixnet routes.");
    println!("The client sends RTT probe traffic through every candidate route,");
    println!("and the RTT analyzer collects per-route statistics in the background.\n");

    println!("Main commands (from the RTT menu):\n");

    println!("  1) Print global RTT statistics");
    println!("     Prints one summary line per route:");
    println!("       - route index");
    println!("       - packets sent (including retransmissions)");
    println!("       - number of ACKs");
    println!("       - number of timeouts");
    println!("       - average RTT (computed over all stored RTT samples, in ms)");
    println!();

    println!("  2) Write stats to CSV file");
    println!("     Writes one line per route to a CSV file on disk.");
    println!("     Current CSV columns:");
    println!("       route,sent,acks,timeouts,avg_rtt");
    println!("         route     : numeric route index");
    println!("         sent      : how many FragmentSent events were recorded");
    println!("         acks      : how many FragmentAckReceived events were recorded");
    println!("         timeouts  : how many FragmentAckExpired events were recorded");
    println!("         avg_rtt   : average RTT (in milliseconds) from all RTT samples");
    println!();

    println!("  3) Print route details BY ROUTE INDEX");
    println!("     Input: a 0-based route index.");
    println!("     Output for that route:");
    println!("       - node list (base58 identities) in order: Node1 > Node2 > Node3");
    println!("       - ALL RTT samples recorded for that route (each sample shown in ms)");
    println!("     This is useful when you already know the route index and");
    println!("     want to inspect exactly how it behaves packet by packet.");
    println!();

    println!("  4) Print route details BY NODE STRING");
    println!("     Input format must match exactly what the analyzer stored, for example:");
    println!("       <node1_base58> > <node2_base58> > <node3_base58>");
    println!("     If a route with that node sequence exists, the tool will:");
    println!("       - print the matching route index");
    println!("       - print the full per-route detail (same as option 3).");
    println!("     This is useful when you have a specific mixnode combination");
    println!("     (e.g. a slow or suspicious path) and want its statistics.");
    println!();

    println!("  5) Print routes with AVERAGE RTT ABOVE a threshold");
    println!("     You provide a threshold in milliseconds (e.g. 150).");
    println!("     The tool will:");
    println!("       - compute avg RTT for each route");
    println!("       - select only routes where avg RTT > threshold");
    println!("       - print detailed info for each matching route (nodes + RTT samples).");
    println!("     Use this to quickly find generally slow routes.");
    println!();

    println!("  6) Print routes with ANY RTT ABOVE a threshold");
    println!("     You provide a threshold in milliseconds (e.g. 500).");
    println!("     For each route, if at least one RTT sample exceeds the threshold,");
    println!("     that route is printed with full details.");
    println!("     Use this to find routes that occasionally spike very high,");
    println!("     even if their average RTT is still acceptable.");
    println!();

    println!("  7) Show experiment progress (percentage completed)");
    println!("     Uses the stored experiment configuration (total_routes, packets_per_route)");
    println!("     plus the number of RTT samples recorded so far to estimate:");
    println!("       completion = received_samples / (total_routes * packets_per_route)");
    println!("     The result is printed as a percentage (0%–100%).");
    println!("     This tells you roughly how far the RTT experiment has progressed.");
    println!();

    println!("  8) Write stats AND generate histogram(s) via Python");
    println!("     This command will:");
    println!("       1) Write the current route statistics to a CSV file (same as option 2).");
    println!("       2) Call the Python script 'rtt_histogram.py' to visualize RTTs.");
    println!();
    println!("     When prompted, you will provide two things:");
    println!("       - CSV file path (where to save the stats)");
    println!("       - outlier_mode string, which controls which histograms are generated:");
    println!();
    println!("         • \"all\"");
    println!("             Use ALL avg_rtt values from the CSV.");
    println!(
        "             Result: a single histogram containing every route's avg RTT (in seconds)."
    );
    println!();
    println!("         • \"<cutoff>\" (numeric, in seconds, e.g. \"1.0\")");
    println!("             Only keep avg_rtt <= cutoff.");
    println!("             Result: a single histogram with INLIERS only (values <= cutoff).");
    println!(
        "             Example: \"1.0\" keeps everything at or below 1.0s and drops slower routes."
    );
    println!();
    println!("         • \"both:<cutoff>\" (e.g. \"both:1.0\")");
    println!("             Split the data into two sets:");
    println!("               - inliers  : avg_rtt <= cutoff");
    println!("               - outliers : avg_rtt >  cutoff");
    println!("             Result: TWO histograms are generated:");
    println!("               1) Distribution of inliers");
    println!("               2) Distribution of outliers");
    println!(
        "             This helps visually compare the \"normal\" routes and the very slow ones."
    );
    println!();

    println!("Helpful notes:");
    println!("  • RTT samples are computed when a FragmentReceived event arrives.");
    println!("    For each fragment that may be retransmitted, the analyzer stores");
    println!("    multiple send times and receive times, and pairs them in order");
    println!("    to compute multiple RTT values for that fragment if needed.");
    println!("  • \"sent\" in the stats includes retransmissions as well, so it may be");
    println!("    higher than packets_per_route for unstable routes.");

    println!("===========================================================\n");
}
