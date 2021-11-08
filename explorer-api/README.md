Network Explorer API
====================

An API that provides data for the [Network Explorer](../explorer).

Features:

  - geolocates mixnodes using https://freegeoip.app/
  - calculates how many nodes are in each country
  - proxies mixnode API requests to add HTTPS
  
## Running

Supply the environment variable `GEO_IP_SERVICE_API_KEY` with a key from https://freegeoip.app/.

Run as a service and reverse proxy with `nginx` to add `https` with Lets Encrypt.

# TODO / Known Issues

## TODO

* record the number of mixnodes on a given date and write to a file for later retrieval
* dependency injection
* tests
