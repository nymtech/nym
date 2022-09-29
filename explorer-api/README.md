Network Explorer API
====================

An API that provides data for the [Network Explorer](../explorer).

Features:

  - geolocates mixnodes using https://app.ipbase.com/
  - calculates how many nodes are in each country
  - proxies mixnode API requests to add HTTPS
  
## Running

Supply the environment variable `GEOIP_DATABASE_PATH` with a path
that points to a GeoIP2 database file in binary format.

Run as a service and reverse proxy with `nginx` to add `https` with Lets Encrypt.

Setup nginx to inject the request IP to the header `X-Real-IP`.

Use https://github.com/maxmind/geoipupdate to automatically
provide and update the GeoIP2 database file.

# TODO / Known Issues

## TODO

* record the number of mixnodes on a given date and write to a file for later retrieval
* dependency injection
* tests
