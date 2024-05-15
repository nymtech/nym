# Performance Monitoring & Testing

Nym Mixnet has been running on mainnet for quite some time. There is still work to be done in order for the network to meet its full potential - mass adoption of privacy through fully distributed Mixnet.

As developers we need to be constantly improving the software. Operators have as much important role, keep their nodes up to date, monitor their performance and share their feedback with the rest of the community and core developers.

Therefore [monitoring](#monitoring) and [testing](#testing) are essential pieces of our common work. We call out all Nym operators to join the efforts! 

## Monitoring

There are multiple ways to monitor performance of nodes and the machines on which they run. For the purpose of maximal privacy and decentralisation of the data - preventing Nym Mixnet from any global adversary takeover - we created these pages as a source of mutual empowerment, a place where operators can share and learn new skills to **setup metrics monitors on their own infrastructure**.

### Guides to Setup Own Metrics

A list of different scripts, templates and guides for easier navigation:

* [`nym-gateway-probe`](gateway-probe.md) - a useful tool used under the hood of [harbourmaster.nymtech.net](https://harbourmaster.nymtech.net)
* [Prometheus and Grafana](prometheus-grafana.md) self-hosted setup
* [Nym-node CPU cron service](https://gist.github.com/tommyv1987/97e939a7adf491333d686a8eaa68d4bd) - an easy bash script by Nym core developer [@tommy1987](https://gist.github.com/tommyv1987), designed to monitor a CPU usage of your node, running locally
* Nym's script [`prom_targets.py`](https://github.com/nymtech/nym/blob/develop/scripts/prom_targets.py) - a useful python program to request data from API and can be run on its own or plugged to more sophisticated flows

### Collecting Testing Metrics

For the purpose of the performance testing Nym core developers plan to run instances of Prometheus and Grafana connected to Node explorer in the house. The network overall key insights we seek from these tests are primarily internal. We're focused on pinpointing bottlenecks, capacity loads, and monitoring cpu usage on the nodes' machines.


## Testing 

```admonish info
For the moment we paused Fast and Furious `perf` environment. Nym Mainnet environment will be used for future tests, please wait for further instructions. 
```

Nym asks its decentralised community of operators to join a series of performance testing events in order to **increase the overall quality of the Mixnet**. The main takeaways of such event are:

1. Understanding of the network behavior under full load
    - How many mixnet client users can all active set entry gateways handle simultaneously?
    - How much sustained IP traffic can a subset of mainnet nodes sustain?
2. Needed improvements of Nym Node binaries to improve the throughput on mainnet
3. Measurement of required machine specs
4. Raw data record
5. Increase quality of Nym Nodes
6. Show each operator a way to monitor their nodes in a distributed fashion

Visit [Fast and Furious web page]({{performance_testing_webpage}}) and [Nym Harbour Master](https://harbourmaster.nymtech.net/) Gateways monitoring page to read more about the performance testing and the results of it.

## Performance Testing Work Flow

* Nym runs a paralel network environment [validator.performance.nymte.ch]({{performance_validator}}) with a chain ID `perf`
* Operators of Nym Nodes join by following easy steps on [performance testing web page]({{performance_testing_webpage}}), including simplified node authentication signature (while keep running their nodes on the mainnet)
* Once signed in, operators will be asked to swap their binary for the modified version with metrics endpoint to be able to connect their own [monitoring system](#monitoring)
* Core node data will be fed to a unique mixnet contract for the `perf` side chain
* Nym starts a new API and start packet transition in high load through these nodes in both settings
* Nym tracks packet flow using Prometheus and Grafana
* Nym creates a large number of clients to the [performance validator network]({{performance_validator}}), intensifying the packet traffic
* Observe and record the metrics - this will probably put more strain on mainnet as well as many nodes spec is minimal

## More Information

* What happens after the test or what operators get for participating is shared up to date on the [performance testing web page]({{performance_testing_webpage}})


