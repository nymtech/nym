# Metrics of Performance Testing

We at Nym as well as several core community operators had setup different metrics flows. To aggregate data into one database would be the easiest way to go. This approach would defeat the very foundation of distributed network run by operators all over the world.

It can be useful for a one time event or limited time period to use the existing metric infrastructure setup by Nym or other operators but for the purpose of maximal privacy and decentralisation of the data - preventing Nym Mixnet from any global adversary takeover - we would like to use these pages as mutual empowerment, a place where operators can share and learn new skills to **[setup metrics monitors on their own infrastructure](#guides-to-setup-own-metrics)**.

## Collecting Testing Metrics

For the purpose of performance test Nym core developers plan to run instances of Prometheus and Grafana in the house. The key insights we seek from these speed tests are primarily internal. We're focused on pinpointing bottlenecks, capacity loads, and monitoring cpu usage on the nodes' machines.

### Community Monitoring Tools

```admonish warning
We would like to encourage operators to make sure they understand and properly evaluate what degree of control they give permission to before granting access to anyones servers.
```
**ExploreNym**

Long term involved operator Pawnflake, an author of [ExploreNYM](https://explorenym.net/) explorer setup his own monitoring flow, which can be used by other operators as well.

A simple guide called `self-hosted-monitor` can be found in their [repository](https://github.com/ExploreNYM/self-hosted-monitor). It utilises bash scripts to enable operators setup [Prometheus](https://github.com/ExploreNYM/self-hosted-monitor/blob/main/prometheus.sh) and [Grafana](https://github.com/ExploreNYM/self-hosted-monitor/blob/main/grafana.sh) together with [Node Exporter](https://github.com/ExploreNYM/self-hosted-monitor/blob/main/node-exporter.sh) and [Nginx](https://github.com/ExploreNYM/self-hosted-monitor/blob/main/nginx-certbot.sh) to run setup their metrics monitoring stack locally.


ExploreNYM also has a similar instance running on their serve called [`enym-monitor`](https://github.com/ExploreNYM/enym-monitor). This setup is very simple for users, however it means that their data are aggregataded all on one server and that is a design we would like to discourage from.

## Guides to Setup Own Metrics

A list of different scripts, templates and guides for easier navigation:

* [Nym-node CPU cron service](https://gist.github.com/tommyv1987/97e939a7adf491333d686a8eaa68d4bd) - an easy bash script by Nym core developer [@tommy1987](https://gist.github.com/tommyv1987), designed to monitor a CPU usage of your node, running locally.
* Nym's script [`prom_targets.py`](https://github.com/nymtech/nym/blob/promethus-is-our-friend/scripts/prom_targets.py) is a useful python program to request data from API and can be run on its own or plugged to more sophisticated flows.
* [Prometheus and Grafana](prometheus-grafana.md) self-hosted setup
