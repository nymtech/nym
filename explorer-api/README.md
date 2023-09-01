# Network Explorer API

An API that provides data for the [Network Explorer](../explorer).

Features:

- geolocates mixnodes using https://dev.maxmind.com/geoip/geolite2-free-geolocation-data
- calculates how many nodes are in each country
- proxies mixnode API requests to add HTTPS

## Development

Several environment variables are required. They can be
provisioned via a `.env` file. For convenience a `.env.dev` is
provided, just copy its content into `.env`.

Follow the steps to setup the geoip database.

## GeoIP db install/update

A geoip database needs to be installed.

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

Note: As mentioned above the explorer-api binary reads the provided `.env` file.

Run as a service and reverse proxy with `nginx` to add `https` with Lets Encrypt.

Setup nginx to inject the request IP to the header `X-Real-IP`.

# TODO / Known Issues

## TODO

- record the number of mixnodes on a given date and write to a file for later retrieval
- dependency injection
- tests
