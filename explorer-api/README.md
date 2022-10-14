# Network Explorer API

An API that provides data for the [Network Explorer](../explorer).

Features:

- geolocates mixnodes using https://dev.maxmind.com/geoip/geolite2-free-geolocation-data
- calculates how many nodes are in each country
- proxies mixnode API requests to add HTTPS

## Running

First you need to provide an `.env` file including **all the
environment variables** listed in `.env.sample`.

When starting the explorer-api, supply the environment variable
`GEOIP_DB_PATH`, pointing to the GeoLite2 binary database file.

For example, in dev env:

```shell
GEOIP_DB_PATH=./geo_ip/GeoLite2-Country.mmdb cargo run
```

Run as a service and reverse proxy with `nginx` to add `https` with Lets Encrypt.

Setup nginx to inject the request IP to the header `X-Real-IP`.

Follow the next section.

## GeoIP db install/update

We use https://github.com/maxmind/geoipupdate to automatically
download and update GeoLite2 binary database.

Supposed you provided an `.env` file with all the required env
variables, simply run the service through docker:

```shell
docker compose up -d geoipupdate
```

# TODO / Known Issues

## TODO

- record the number of mixnodes on a given date and write to a file for later retrieval
- dependency injection
- tests
