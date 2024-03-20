# Node Setup for Performance Testing Event

To join the [Performance testing event]({{performance_testing_webpage}}) node operators need to do proceed with the following tasks:

1. [Sign their node]({{performance_testing_webpage}}) into the testing environment
2. [Configure their node](#node-configuration) for the test
3. (Optional) [Setup metric monitoring system](templates.md) to observe node performance at any time

## Node Configuration

> Any syntax in `<>` brackets is a user's unique variable/version. Exchange with a corresponding name without the `<>` brackets.

After you signed your node (or several) into the Performance testing environment, open the machine with (each of) your nodes and follow the steps below to configure your node for the event.


1. Download and setup `2024.2-fast-and-furious` binary from our [release page](https://github.com/nymtech/nym/releases/)
  - Follow the steps to upgrade node on the [maintenance page](../nodes/manual-upgrade.md)
  - Before you start your node on the binary, follow the steps below

2. If you run `nym-gateway` proceed with these steps. If not, go to the next point
  - Make sure to have your `nym-gateway` setup as [Nym Exit Gateway](../legal/exit-gateway.md) following the commands [here](../nodes/gateway-setup.md#initialising-exit-gateway)
  - Enable `[ip_packet_router]` (IPR) in your `~/.nym/gateways/*/config/config.toml` with the command below and then run [this script](https://gist.github.com/tommyv1987/ccf6ca00ffb3d7e13192edda61bb2a77) to enable IPv4 and IPv6
```sh
# command to enable IPR
./nym-gateway setup-ip-packet-router --id "nym-gateway" --enabled true

# script to enable IPv4 and IPv6
curl -o enable_networking_for_nym_nodes.sh https://gist.githubusercontent.com/tommyv1987/ccf6ca00ffb3d7e13192edda61bb2a77/raw/0840e1d2ee9949716c45655457d198607dfd3107/enable_networking_for_nym_nodes.sh -L && chmod u+x enable_networking_for_nym_nodes.sh && sudo ./enable_networking_for_nym_nodes.sh
```

3. If you run Prometheus for [monitoring](templates.md) add a `<NODE_METRICS_KEY>` to your node `config.toml` by running [this script](https://gist.github.com/benedettadavico/1299b2c7b8b8282c15eafb1914fb3594) with an arbitrary `<NODE_METRIC_KEY>` of your own choice as an argument, follow these commands with your own **strong passphrase**
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
  - Add this `<NODE_METRIC_KEY>` string to your monitoring Prometheus config `prometheus.yml` as a value to `credentials` as below

```yaml
scrape_configs:
  # The job name is added as a label `job=<job_name>` to any timeseries scraped from this config.
  - job_name: "prometheus"
    authorization:
      credentials: <METRICS_KEY_SET_ON_THE_NODE>

    static_configs:
      - targets: ["localhost:9090"]

    file_sd_configs:
    - files:
      - /tmp/prom_targets.json
```
  - Open ports for scraping the metrics
```sh
sudo ufw allow 9000, 9001
```
4. Restart your node, in case you run it as a [`systemd` service](../nodes/maintenance.md#systemd) make sure to run `systemctl daemon-reload` before the restart
  - Make sure your node runs with root privileges, either in a root shell or with `sudo` command


<!--
TODO:
- Changes on binary side of things - create a binary release solely for mixnodes (release/2024.2-fast-and-furious) - have set it up as a pre-release and that only (in process)
- investigate if https://github.com/nymtech/nym/pull/4474 can work alongside ipr as a backup (it will be good to kill all birds with one stone here)
- more regression testing to do on the env, to ensure no blacklisting of gateways/mixnodes and ensure the env is behaving correctly + document it
-->
