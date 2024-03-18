# Metrics of Performance Testing

At Nym as well as several core community operators had setup metrics monitors for a clear overview of node performance.

It is benefitial for a one time event or a limited period to connect nodes to the existing monitoring infrastructure system that Nym developers built to collect useful metrics. For the purpose of maximal privacy and decentralisation of the data - preventing Nym Mixnet from any global adversary takeover - we created these pages as a source of mutual empowerment, a place where operators can share and learn new skills to **setup metrics monitors on their own infrastructure**.

## Collecting Testing Metrics

For the purpose of the performance testing Nym core developers plan to run instances of Prometheus and Grafana connected to Node explorer in the house. The network overall key insights we seek from these tests are primarily internal. We're focused on pinpointing bottlenecks, capacity loads, and monitoring cpu usage on the nodes' machines.

## Guides to Setup Own Metrics

A list of different scripts, templates and guides for easier navigation:

* [Prometheus and Grafana](prometheus-grafana.md) self-hosted setup
* [Nym-node CPU cron service](https://gist.github.com/tommyv1987/97e939a7adf491333d686a8eaa68d4bd) - an easy bash script by Nym core developer [@tommy1987](https://gist.github.com/tommyv1987), designed to monitor a CPU usage of your node, running locally.
* Nym's script [`prom_targets.py`](https://github.com/nymtech/nym/blob/promethus-is-our-friend/scripts/prom_targets.py) is a useful python program to request data from API and can be run on its own or plugged to more sophisticated flows.

