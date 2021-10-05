Network Explorer API
====================

An API that can: 

* calculate how many nodes are in which country, by checking the IPs of all nodes against an external service
* serve "hello world" via HTTP


TODO:

* record the number of mixnodes on a given date and write to a file for later retrieval
* store the nodes per country state in a variable
* grab mixnode description info via reqwest and serve it (avoid mixed-content errors)
* serve it all over http
* dependency injection
* tests

## Running

- Supply the environment variable `GEO_IP_SERVICE_API_KEY` with a key from https://freegeoip.app/