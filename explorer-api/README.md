# Network Explorer API

An API that provides data for the [Network Explorer](../explorer).

Features:

- geolocates mixnodes using https://dev.maxmind.com/geoip/geolite2-free-geolocation-data
- calculates how many nodes are in each country
- proxies mixnode API requests to add HTTPS

## GeoIP db install/update

First we need to install the geoip database.

We use https://github.com/maxmind/geoipupdate to automatically
download and update GeoLite2 binary database. For convenience we
run it as a docker container.

At the root of the repo, inside the `docker-compose.yml`, there
is a docker service `geoipupdate` for it.

Supposed you provided an `.env` file with **all the environment
variables** listed in `.env.sample-dev` (found at the root),
simply run the service through docker:

```shell
docker compose up -d geoipupdate
```

Running this command will automatically install (and update) the
db file inside the directory path provided by `GEOIP_DB_DIRECTORY`
env variable.

## Running

When starting the explorer-api, supply the environment variable
`GEOIP_DB_PATH`, pointing to the GeoLite2 binary database file.
It should be previously installed thanks to `geoipupdate` service.

For example:

```shell
GEOIP_DB_PATH=./geo_ip/GeoLite2-Country.mmdb cargo run
```

Note: explorer-api binary reads the provided `.env` file.

Run as a service and reverse proxy with `nginx` to add `https` with Lets Encrypt.

Setup nginx to inject the request IP to the header `X-Real-IP`.

# TODO / Known Issues

## TODO

- record the number of mixnodes on a given date and write to a file for later retrieval
- dependency injection
- tests
