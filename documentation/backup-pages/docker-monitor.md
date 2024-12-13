<!--
TODO
1. Add a few points of advantages why to run it in this way (clean, remote from environment -> safer,  can use dockerhub etc etc)
2. Explain in Prometheus setup (in config step) how to get the metrics key and how to add it to the node config (that will be documented in node-setup.md)
3. Add grafana setup
4. Add prometheus setup
5. Add node exporter setup
6. Add how to view it in the browser (possible to use the setup from explorenym guide
7. Additionally add how to spin up an ansible template
-->

# Setup Prometheus and Grafana in a Docker

This setup may seem more complex than running a few scripts, but it has certain advantages.
- Running things in a container
<!-- ADD DVANTEGES OF DOCKER SETUP -->

## Docker, Prometheus and Grafana Metric SETUP

This setup can be done from the same VPS like your node or on a completelly remote machine. In any case it's setup in a Docker container so it works remotely from the environment where your node runs.

### Docker setup

<!--INIT AND START DOCKER CONTAINER -->

### Prometheus Setup

This entire installation shall be done with `root` privileges. If you not `root`, start with `su` command before the following steps.

1. Get the latets system updates
```sh
apt update
```

2. Create a system user for Prometheus
```sh
groupadd --system prometheus
useradd -s /sbin/nologin --system -g prometheus prometheus
```

3. Create directories for Prometheus
```sh
mkdir /etc/prometheus
mkdir /var/lib/prometheus
```

4. Download and extract Prometheus
```sh
wget https://github.com/prometheus/prometheus/releases/download/v{{prometheus_latest_version}}/prometheus-{{prometheus_latest_version}}.linux-amd64.tar.gz
tar vxf prometheus-{{prometheus_latest_version}}.linux-amd64.tar.gz
```
In case of errors, check Prometheus [release page](https://github.com/prometheus/prometheus/releases/) and get the correct binary for your system.

5. Navigate to Prometheus directory and configure Prometheus
```sh
# change directory
cd prometheus-{{prometheus_latest_version}}.linux-amd64

# move the binary files
mv prometheus /usr/local/bin
mv promtool /usr/local/bin

# set owner
chown prometheus:prometheus /usr/local/bin/prometheus
chown prometheus:prometheus /usr/local/bin/promtool

# move the config files
mv consoles /etc/prometheus
mv console_libraries /etc/prometheus
mv prometheus.yml /etc/prometheus


# set owner
chown prometheus:prometheus /etc/prometheus
chown -R prometheus:prometheus /etc/prometheus/consoles
chown -R prometheus:prometheus /etc/prometheus/console_libraries
chown -R prometheus:prometheus /var/lib/prometheus
```

6. Open the main Prometheus config file `prometheus.yml`
```sh
nano /etc/prometheus/prometheus.yml
```

7. Paste the block below to your config `prometheus.yml`, change the line `credentials` and save it (`ctrl` + `x`)
    - `credentials` value must be the same like in your node `config.toml` config file under `[http]` header
    - In case you haven't set up yet a `<NODE_METRICS_KEY>` in your node `config.toml`, add it by opening a machine with your node and run [this script](https://gist.github.com/benedettadavico/1299b2c7b8b8282c15eafb1914fb3594) with an arbitrary `<NODE_METRIC_KEY>` of your own choice as an argument, follow these commands with your own **strong passphrase**
```sh
# get the script
curl -L https://gist.githubusercontent.com/benedettadavico/1299b2c7b8b8282c15eafb1914fb3594/raw/500c36037615a515f2f3e007baa25e6a2c277d4a/update_config.sh -o update_config.sh

# make executable
chmod u+x ./update_config.sh

# run with your own key as argument
sh ./update_config.sh <NODE_METRIC_KEY>

# for example if you chose my passhphrase to be: "makemoresecurekeythanthis1234"
# the command would look like this
# sh ./update_config.sh makemoresecurekeythanthis1234
```
  - Add this `<NODE_METRIC_KEY>` string to your monitoring Prometheus config `prometheus.yml` as a value to `credentials`, paste the block below to your `prometheus.yml` as below

```yaml
# my global config
global:
  scrape_interval: 15s # Set the scrape interval to every 15 seconds. Default is every 1 minute.
  evaluation_interval: 15s # Evaluate rules every 15 seconds. The default is every 1 minute.
  # scrape_timeout is set to the global default (10s).

# Alertmanager configuration
alerting:
  alertmanagers:
    - static_configs:
        - targets:
          # - alertmanager:9093

# Load rules once and periodically evaluate them according to the global 'evaluation_interval'.
rule_files:
  # - "first_rules.yml"
  # - "second_rules.yml"

# A scrape configuration containing exactly one endpoint to scrape:

scrape_configs:
  # The job name is added as a label `job=<job_name>` to any timeseries scraped from this config.
  - job_name: "prometheus"
    authorization:
    # CHANGE THIS TO YOUR OWN STRING
      credentials: <METRICS_KEY_SET_ON_THE_NODE>

    static_configs:
      - targets: ["localhost:9090"]

    file_sd_configs:
    - files:
      - /tmp/prom_targets.json
```

8. Create Prometheus systemd service by saving the block below to as `/etc/systemd/system/prometheus.service`:

```sh
nano /etc/systemd/system/prometheus.service
```

```sh
[Unit]
Description=Prometheus
Wants=network-online.target
After=network-online.target

[Service]
User=prometheus
Group=prometheus
Type=simple
ExecStart=/usr/local/bin/prometheus \
    --config.file /etc/prometheus/prometheus.yml \
    --storage.tsdb.path /var/lib/prometheus/ \
    --web.console.templates=/etc/prometheus/consoles \
    --web.console.libraries=/etc/prometheus/console_libraries

[Install]
WantedBy=multi-user.target
```

9. Reload, enable, start and check Prometheus service
```sh
systemctl daemon-reload
systemctl enable prometheus
systemctl start prometheus
systemctl status prometheus

# to observe journal log, run
journalctl -f -u prometheus
```

10. Open port for Prometheus web interface
```sh
ufw allow 9090/tcp
```
11. Finally you can access Prometheus on `localhost:9090` or `<IP_ADDRESS>:9090`

Further reading on Prometheus functionalities:
- [Alerting overview](https://prometheus.io/docs/alerting/latest/overview/)
- [Exporters and Integration](https://prometheus.io/docs/instrumenting/exporters/)

### Grafana Setup

<!-- SETUP GRAFANA IN A DOCKER -->

## Setup Access on your Node

This part needs to be done on the machine where your node runs

### Node Exporter Setup
<!-- PROMETHEUS NODE EXPORTER-->


## Grafana Dashboard Access

Finally we need to access Grafana dashboards.

1. Open a browser at `http://<YOUR_IP_ADDRESS>:3000` or `https://<YOUR_DOMAIN>` (depends on your setup), enter username `admin` and password `admin` and setup new credentials on prompt

2. Setup *Data source* by opening menu -> `Connections` -> `Data sources` -> `+ Add new data source` -> `Prometheus`

![](../images/grafana/add-data-sources.png)
![](../images/grafana/add-data-source-prometheus.png)

3. In the field *Connection* next to `Prometheus server URL` enter `http://localhost:9090` (regardless if you accessing Grafana via `http` or `https` as this is for internal connection on the server). When you are done in the bottom confirm by `Save & Test`

4. In the menu open: `Dashboards` -> `+ Create dashboard` -> `Import dashboard`

![](../images/grafana/import-dashboard.png)

5. ID field: enter `1860` -> `Load`

![](../images/grafana/id-1860.png)

6. In *Import dashboard* page select Prometheus in the bottom and finally `Import`

![](../images/grafana/add-prometheus.png)

Now you have your Prometheus panels displayed via Grafana dashboard for a simple monitoring of your node.

<!--
## Verification and Troubleshooting

To ensure that your services are running correctly, you can verify that by running `systemctl status <SERVICE>` or run a `journalctl -f -u <SERVICE>` to print service logs. It shall return status `Active: active (running). For example:
```sh
# to check if Prometheus service is active
systemctl status prometheus

# to check if Grafana service is active
systemctl status grafana-server

# to check if node-exporter service is active
systemctl status node_exporter

# to run journal log
journalctl -f -u prometheus # or any other service you want to see
```
-->


## Grafana Setup
