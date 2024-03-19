# [ ] Node Setup for Performance Testing Event

To join the [Performance testing event]({{performance_testing_webpage}}) node operators need to do proceed with the following tasks:

1. [Sign their node]({{performance_testing_webpage}}) into the testing environment
2. [Configure their node](#node-configuration) for the test
3. (Optional) [Setup metric monitoring system](templates.md) to observe node performance at any time

## Node Configuration

After you signed your node (or several) into the Performance testing environment, open the machine with (each of) your nodes and follow the steps below to configure your node for the event.

1. Download and setup [`2024.2-fast-and-furious`](URL) binary
```sh
curl foo
chmod foo
init/what???
```


2. Open ports for scraping the metrics
```sh
sudo ufw allow 9000, 9001
```

3. Run the binary with a `run` command or possibly as a [`systemd` service](../nodes/maintenance.md#systemd)
```sh
run foo
```

4. Add a `<NODE_METRICS_KEY>` to your node `config.toml` by running [this script](https://gist.github.com/benedettadavico/1299b2c7b8b8282c15eafb1914fb3594) with an arbitrary `<NODE_METRIC_KEY>` of your own choice as an argument, follow these commands with your own **strong passphrase**
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

<!--
TODO:
- Changes on binary side of things - create a binary release solely for mixnodes (release/2024.2-fast-and-furious) - have set it up as a pre-release and that only (in process)
- investigate if https://github.com/nymtech/nym/pull/4474 can work alongside ipr as a backup (it will be good to kill all birds with one stone here)
- more regression testing to do on the env, to ensure no blacklisting of gateways/mixnodes and ensure the env is behaving correctly + document it
-->
