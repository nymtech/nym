# Nym Network Monitor

Monitors the Nym network by sending itself packages across the mixnet.

Network monitor is running two tokio tasks, one manages mixnet clients and another manages monitoring itself. Monitor is designed to be driven externally, via an HTTP api. This means that it does not do any monitoring unless driven by something like [`locust`](https://locust.io/). This allows us to tailor the load externally, potentially distributing it across multiple monitors.

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
  -p, --port <PORT>                        Port to listen on [default: 8080]
  -h, --host <HOST>                        Host to listen on [default: 127.0.0.1]
  -t, --topology <TOPOLOGY>                Path to the topology file [default: topology.json]
  -h, --help                               Print help
  -V, --version                            Print version
```


