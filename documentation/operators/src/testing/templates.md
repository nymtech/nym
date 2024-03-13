# Metrics of Performance Testing

We at Nym as well as several core community operators had setup different metrics flows. To aggregate data into one database would be the easiest way to go. This approach would defeat the very foundation of distributed network run by operators all over the world.

It can be useful for a one time event or limited time period to use the existing metric infrastructure setup by Nym or other operators but for the purpose of maximal privacy and decentralisation of the data - preventing Nym Mixnet from any global adversary takeover - we would like to use these pages as mutual empowerment, a place where operators can share and learn new skills to **[setup metrics monitors on their own infrastructure](#guides-to-setup-own-metrics)**.

## Collecting Testing Metrics

For the initial performance test Nym core developers plan to run instances of Prometheus and Grafana in the house. The key insights we seek from these speed tests are primarily internal. We're focused on pinpointing bottlenecks, capacity loads, and monitoring cpu usage on the nodes' machines.

### Community Monitoring Tools

```admonish warning
Before granting access to anyones servers we would like to encourage operators to understand what degree of control they committing to.
```

At Nym we always support community builders to create their tools and share with others. While we create connections based on trust and reputation we also recognize the underlying problem with giving permission to data access to *anyone* based on trust.

**ExploreNym**

Long term involved operator Pawnflake, an author of [ExploreNYM](https://explorenym.net/) explorer setup his own monitoring flow, which can be used by other operators as well. A simple guide called `vps-monitor` can be found in their [repository ](https://github.com/ExploreNYM/vps-monitor).

## Guides to Setup Own Metrics

A list of different scripts, templates and guides for easier navigation:

* [Nym-node CPU cron service](https://gist.github.com/tommyv1987/97e939a7adf491333d686a8eaa68d4bd) - an easy bash script by Nym core developer [@tommy1987](https://gist.github.com/tommyv1987), designed to monitor a CPU usage of your node, running locally.
* Nym's script [`prom_targets.py`](https://github.com/nymtech/nym/blob/promethus-is-our-friend/scripts/prom_targets.py) is a useful python program to request data from API and can be run on its own or plugged to more sophisticated flows.
* [Prometheus and Grafana](prometheus-grafana.md)
