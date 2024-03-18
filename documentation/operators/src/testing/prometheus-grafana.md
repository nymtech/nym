# Prometheus & Grafana

The combination of Prometheus and Grafana is a common stack used by Nym team for internal monitoring as well as by core community operators like [ExploreNym](https://github.com/ExploreNYM/vps-monitor) or [No Trust Verify](https://status.notrustverify.ch/d/CW3L7dVVk/nym-mixnet?orgId=1).

<!-- Write about adventages of this setup -->

## Prometheus

[Prometheus](https://prometheus.io) is a free and open source monitoring systems. It allows operators to have their metrics, events, and alerts under full control. This ecosystem offers multiple advantages:

- collects and records metrics from servers, containers, and applications
- provides a [flexible query language (PromQL)](https://prometheus.io/docs/prometheus/latest/querying/basics/)
- multiple modes visualization tools
- an alerting mechanism that sends notifications

Prometheus collects and stores its metrics as time series data, i.e. metrics information is stored with the timestamp at which it was recorded, alongside optional key-value pairs called labels.

## Grafana

[Grafana](https://grafana.com/docs/grafana/latest/) is an open-source analytics and interactive front end. It is widely used for its easy to manage dashboards with visualizations like graphs, charts and alerts, all connected to live data sources.

## Setup Guides

There are various ways how to setup this stack. You can chose based on your preferences to do your own flow or follow some of the documented ones:

- [ExploreNYM scripts](explorenym-scripts.md) for self-hosted monitoring
- Setup monitoring in a Docker container (*detailed guide will be published soon*)
<!--- [Run in a Docker](docker-monitor.md) -->

## References and further reading

* [Prometheus release page](https://prometheus.io/download/)
* [Prometheus documentation](https://prometheus.io/docs/introduction/overview/)
* Installation [guide to install Prometheus](https://www.cherryservers.com/blog/install-prometheus-ubuntu) on Ubuntu by cherryservers
* [Grafana installation guide](https://grafana.com/docs/grafana/latest/setup-grafana/installation/debian/)
* Nym's script [`prom_targets.py`](https://github.com/nymtech/nym/blob/promethus-is-our-friend/scripts/prom_targets.py) - a python program to request data from API and can be plugged to this stack
