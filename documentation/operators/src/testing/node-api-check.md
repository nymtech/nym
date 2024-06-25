# Node API Check

> Any syntax in `<>` brackets is a user's unique variable/version. Exchange with a corresponding name without the `<>` brackets.

Operating a `nym-node` is not a *"set and forget"* endeavor, it takes some work. To diagnose node performance querying APIs is a good knowledge to have. There are two main places to look for API endpoints regarding `nym-node`:

- [`openapi.json`](https://validator.nymtech.net/api/v1/openapi.json): a list of all endpoints
- [Swagger UI page](https://validator.nymtech.net/api/swagger/index.html)

Besides that, Gateway operators can check out their node performance, connectivity and much more on [harbourmaster.nymtech.net](https://harbourmaster.nymtech.net/).

### Basic API usage

For information about available endpoints and their status, you can refer to:
```
# for http
http://<IP>:8080/api/v1/swagger/#/

# for https reversed proxy
https://<DOMAIN>/api/v1/swagger/#/
```

For example to determine which mode your node is running, you can check `:8080/api/v1/roles` endpoint:
```
# for http
http://<IP_ADDRESS>:8080/api/v1/roles

# for https reversed proxy
https://<DOMAIN>/api/v1/roles
```

## `node_api_check.py`

To make this a bit easier, we made a CLI tool quering all vailable API endpoints based on node `Identity Key` (further denoted as `<ID_KEY>`) called `node_api_check.py`. To diagnose your node performance, whether by yourself or by sharing an output in our [operator channel](https://matrix.to/#/#operators:nymtech.chat), this tool provides you with a quick overview of live data. We recommend to run this checker alongside [`nym_gateway_probe`](gateway-probe.md) to triage both performance and an actual routing.

Besides querying any bonded node APIs, `nym_api_check` has a function counting all existing nodes in provided version.

### Setup

#### Pre-requsities

**Python3**

1. Start with installing Python3:
```sh
sudo apt install Python3
```

2. Make sure Python3 is your default Python version:
```sh
update-alternatives --install  /usr/bin/python python /usr/bin/python3 1

# controll
python --version
# should return higher than 3
```

3. Install Python modules `tabulate`, `pandas` and `argparse`:
- either using [`pip`](https://python.land/virtual-environments/installing-packages-with-pip) and then running:
```sh
pip install tabulate pandas argparse
```
- or if you installed Python3 system-wide you can install modules directly:
```sh
sudo apt install Python3-tabulate Python3-pandas Python3-argparse
```

**Installation**

4. Get [`node_api_check.py`](https://github.com/nymtech/nym/tree/develop/scripts/node_api_check.py) and [`api_endpoints.json`](https://github.com/nymtech/nym/tree/develop/scripts/api_endpoints.json). If you [compiled from source](../binaries/building-nym.md), you already have both of these files. If you prefer to download them individually, do it by opening terminal in your desired location and running:
```sh
wget https://raw.githubusercontent.com/nymtech/nym/tree/develop/node_api_check.py

wget https://raw.githubusercontent.com/nymtech/nym/tree/develop/api_endpoints.json
```

5. Make executable:
```sh
chmod u+x node_Api.check.py
```

Now you are ready to check your node.

### Usage

Run with `--help` flag to see the available commands:

~~~admonish example collapsible=true title="./node_api_check.py --help"
```python
<!--cmdrun cd ../../../../scripts && python ./node_api_check.py --help-->
```
~~~

#### `query_stats`

When you want to see all the options connected to a command, add a `--help` flag after the command of your choice, like in this example:
~~~admonish example collapsible=true title="./node_api_check.py query_stats --help"
```python
<!--cmdrun cd ../../../../scripts && python ./node_api_check.py query_stats --help-->
```
~~~

The most common usage may be `./node_api_check.py query_stats <ID_KEY>` where `<ID_KEY>` is required, sustitute it with node Identity Key.

**Optional arguments**

| Flag                   | Shortcut | Description                                                 |
| :---                   |     :---      | :---                                                        |
| `--markdown`           |     `-m`      | returns output in markdown format                           |
| `--no_routing_history` |     `-n`      | returns output without routing history which can be lenghty |
| `--output`             |     `-o`      | exports output to a file, possible to add a target path     |

#### `version_count`

Another command is `version_count` where at least one `nym-node` version is required. In case of multiple version count, separate the versions with space. We recommend to run this command with `--markdown` flag for a nicer output. This is an example where we want to look up how many registered nodes are on versions `1.1.0`, `1.1.1`, `1.1.2` and `1.1.3`:
```sh
./node_api_check version_count 1.1.0 1.1.1 1.1.2 1.1.3 --markdown
```
