# Nym Network Monitor

Monitors the Nym network by sending itself packages across the mixnet.

Network monitor is running two tokio tasks, one manages mixnet client and another manages monitoring itself. Monitor is designed to be driven externally, via an HTTP api. This means that it does not do any monitoring unless driven by something like `locust`. This allows us to tailor the load externally, potentially distributing it across multiple monitors.

### Client manager

On start network monitor will spawn `C` clients, with 10 being the default. Random client is dropped every `T`, defaults to 60 seconds, and a new one is created. Clients chose a random gateway to connect to the mixnet. Meaning that on average all gateways will be tested in `NUMBER_OF_GATEWAYS/N*T`, assuming at least one request per client per T.

### Network monitor API

##  Usage

```bash
Usage: nym-network-monitor [OPTIONS]

Options:
  -c, --clients <CLIENTS>  [default: 10]
  -h, --help               Print help
  -V, --version            Print version
```


