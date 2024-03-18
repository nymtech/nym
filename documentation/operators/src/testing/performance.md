# Performance Testing

Nym Mixnet has been running on mainnet for quite some time already. There is still work to be done in order for the network to meet its full potential - mass adoption of privacy on network level through fully distributed Mixnet.

Nym therefore calls out the decentralised community of operators to join the Performance testing events in order to **overall increase quality of Nym Mixnet**. The main takeaways of such event are:

1. Understanding of the network behavior under full load
    - How many mixnet client users can all active set entry gateways handle simultaneously?
    - How much sustained IP traffic can a subset of mainnet nodes sustain?
2. Needed improvements of Nym Node binaries to improve the throughput on mainnet
3. Measurement of required machine specs
4. Raw data record instead of guessing
5. Increase quality of Nym Nodes
6. Show each operator a way to monitor their nodes in a distributed fashion

## Performance Testing Work Flow

* Nym runs a paralel network environment [validator.performance.nymte.ch]({{performance_validator}}) with a chain ID `perf`
* Operators of Nym Nodes (currently `nym-mixnode` and `nym-gateway`) join by following easy steps on [performance testing web page]({{performance_testing_webpage}}), including simplified node authentication signature (while keep running on mainnet simultaneously)
* Core node data will be fed to a unique mixnet contract for the `perf` side chain
* Once connected, operators will be asked to swap their binary for the modified version with metrics endpoint
* Nym starts a new API and start packet transition in high load through these nodes in both settings
* Nym tracks packet flow through Prometheus and Grafana
* Nym creates a large number of clients to the [performance validator network]({{performance_validator}}), intensifying the packet traffic
* Observe and record the metrics - this will probably put more strain on mainnet as well as many nodes spec is minimal

## More Information

* What happens after the test or what operators get for participating is shared up to date on the [performance testing webpage]({{performance_testing_webpage}})
* Visit our guides to [setup metrics template](templates.md) and learn how to operate them in self-custodial way
