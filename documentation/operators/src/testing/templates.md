# Metrics of Performance Testing

At Nym as well as several core community operators had setup metrics monitors for a clear overview of node performance.

It is useful for a one time event or limited period to use the existing metric infrastructure stack  Nym developers. For the purpose of maximal privacy and decentralisation of the data - preventing Nym Mixnet from any global adversary take-over - we created these pages as a source of mutual empowerment, a place where operators can share and learn new skills to **setup metrics monitors on their own infrastructure**.

## Collecting Testing Metrics

For the purpose of performance test Nym core developers plan to run instances of Prometheus and Grafana in the house. The network overall key insights we seek from these tests are primarily internal. We're focused on pinpointing bottlenecks, capacity loads, and monitoring cpu usage on the nodes' machines.

### Community Monitoring Tools

Individual operators, noide families and squads are the foundation of distributed network. There has been a great number of tools coming out of this community.

```admonish warning
Make sure you understand and properly evaluate what degree of control you give permission to before granting access to your data to any tools running on someone else's servers.
```
**ExploreNym**

Long term involved operator Pawnflake, an author of [ExploreNYM](https://explorenym.net/) explorer setup a monitoring flow, which can be used by other operators called [`self-hosted-monitor`](https://github.com/ExploreNYM/self-hosted-monitor). It utilises bash scripts to enable operators setup [Prometheus](https://github.com/ExploreNYM/self-hosted-monitor/blob/main/prometheus.sh) and [Grafana](https://github.com/ExploreNYM/self-hosted-monitor/blob/main/grafana.sh) together with [Node Exporter](https://github.com/ExploreNYM/self-hosted-monitor/blob/main/node-exporter.sh) and [Nginx](https://github.com/ExploreNYM/self-hosted-monitor/blob/main/nginx-certbot.sh) to run setup their metrics monitoring stack locally.

In collaboration with ExploreNYM we published a [step by step guide](


ExploreNYM also has a network measuring instance called `enym-monitor`. This setup is very simple for users, however it means that their data are aggregataded all on one  server and that is a design we would like to discourage from.

## Guides to Setup Own Metrics

A list of different scripts, templates and guides for easier navigation:

* [Nym-node CPU cron service](https://gist.github.com/tommyv1987/97e939a7adf491333d686a8eaa68d4bd) - an easy bash script by Nym core developer [@tommy1987](https://gist.github.com/tommyv1987), designed to monitor a CPU usage of your node, running locally.
* Nym's script [`prom_targets.py`](https://github.com/nymtech/nym/blob/promethus-is-our-friend/scripts/prom_targets.py) is a useful python program to request data from API and can be run on its own or plugged to more sophisticated flows.
* [Prometheus and Grafana](prometheus-grafana.md) self-hosted setup
