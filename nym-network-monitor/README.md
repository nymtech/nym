# Nym Network Monitor

Monitors the Nym network by sending itself packages across the mixnet.

Network monitor is running two tokio tasks, one manages mixnet clients and another manages monitoring itself. Monitor is designed to be driven externally, via an HTTP api. This means that it does not do any monitoring unless driven by something like [`locust`](https://locust.io/). This allows us to tailor the load externally, potentially distributing it across multiple monitors.

## Features

- **Continuous Monitoring**: Periodically sends test packets through the network
- **Node Performance**: Tracks individual node reliability metrics
- **Route Performance**: Records route-level success rates through specific node combinations
- **Multi-API Submission**: Capable of submitting metrics to multiple API endpoints (fanout)
- **Force Routing**: Can force packets through all mixnet nodes for comprehensive testing

### Client manager

On start network monitor will spawn `C` clients, with 10 being the default. Random client is dropped every `T`, defaults to 60 seconds, and a new one is created. Clients chose a random gateway to connect to the mixnet. Meaning that on average all gateways will be tested in `NUMBER_OF_GATEWAYS/N*T`, assuming at least one request per client per T.

### Network monitor API

Swagger UI is available at `/v1/ui/`, ie `http://localhost:8080/v1/ui/`

### Driving the monitor with Locust

+ Head over to https://locust.io/ and get `locust`
+ Start everything
```bash
# Start the network monitor
cargo run --release

# Start locus in a separate terminal
python -m locust -H http://127.0.0.1:8080 --processes 4
```
+ Head over to http://127.0.0.1:8089/ and start a testing run

##  Usage

```bash
Usage: nym-network-monitor [OPTIONS]

Options:
  -C, --clients <N_CLIENTS>                Number of clients to spawn [default: 10]
  -T, --client-lifetime <CLIENT_LIFETIME>  Lifetime of each client in seconds [default: 60]
      --port <PORT>                        Port to listen on [default: 8080]
      --host <HOST>                        Host to listen on [default: 127.0.0.1]
  -t, --topology <TOPOLOGY>                Path to the topology file
  -e, --env <ENV>                          Path to the environment file
  -m, --mixnet-timeout <MIXNET_TIMEOUT>    [default: 10]
      --generate-key-pair
      --private-key <PRIVATE_KEY>
      --database-url <DATABASE_URL>        SQLite database URL
      --nym-apis <NYM_APIS>                Comma-separated list of Nym API URLs
  -h, --help                               Print help
  -V, --version                            Print version
```

## Metrics Collection & Reporting

### Node Metrics

The Network Monitor tracks performance metrics for individual nodes:

- **Reliability**: Percentage of successful packet handling
- **Failure Sequences**: Tracking consecutive failures
- **Volume**: Number of packets handled

### Route Metrics

Since version 1.1.0, the Network Monitor also tracks route-level metrics:

- **Route Success Rates**: Tracking which specific combinations of nodes have successful packet delivery
- **Layer Analysis**: Identifying weak points in specific network layers
- **Path Correction**: Improved algorithms for attributing failures to specific nodes

### Metrics Fanout

The Network Monitor can submit metrics to multiple API endpoints simultaneously:

1. Metrics are collected during each monitoring cycle
2. The collected metrics are submitted to each configured API endpoint
3. This provides redundancy and allows for distributed metrics collection

To enable metrics fanout, use the `--nym-apis` parameter with a comma-separated list of API URLs:

```bash
cargo run -p nym-network-monitor -- --nym-apis https://api1.example.com,https://api2.example.com
```

## Route Data Structure

Route metrics use the following data structure:

```rust
// Route performance data
pub struct RouteResult {
    pub layer1: u32,     // NodeId of layer 1 mixnode
    pub layer2: u32,     // NodeId of layer 2 mixnode
    pub layer3: u32,     // NodeId of layer 3 mixnode
    pub gw: u32,         // NodeId of gateway 
    pub success: bool,   // Whether the packet was successfully delivered
}
```

## Forced Routing

To ensure comprehensive testing of all nodes in the network, the Monitor supports forcing packets through all available nodes:

- Each node is assigned to a specific layer (1, 2, or 3) deterministically
- This ensures all nodes participate in route testing
- The routing algorithm cycles through all possible node combinations

Since version 1.1.0, Network Monitor automatically forces all available nodes to be active and distributes them evenly across the three layers (Layer 1, Layer 2, and Layer 3). This ensures every node in the network participates in testing, providing more comprehensive coverage and better metrics for all nodes, not just the popular ones.

## Node Performance Calculation

The Network Monitor uses a sophisticated algorithm for attributing failures to specific nodes:

1. For successful packet deliveries, all nodes in the path receive a positive sample
2. For failed deliveries:
   - Nodes with more than 2 consecutive failures are considered "guilty"
   - If no node is clearly guilty, all nodes in the path receive negative samples
3. Final node reliability is calculated as: positive_samples / (positive_samples + negative_samples)

## Changelog

### Version 1.1.0
- Added route-level metrics tracking and submission
- Implemented metrics fanout to multiple API endpoints
- Forced routing through all available nodes for comprehensive testing
- Improved reliability corrections with consecutive failure tracking
- Updated data structures for better metrics organization

### Version 1.0.2
- Initial public release with basic monitoring capabilities