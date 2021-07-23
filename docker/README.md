## Build with Docker & Docker Compose

Currently you can build and run locally in a Docker containers the following components of the Nym Privacy Platform:

* One genesis validator
* Any number of secondary validators
* A contract uploader, that uploads the contract built from the local sources
* The web wallet application, accessible on port 3000
* The block explorer application, accessible on port 3080
* The network explorer application, accessible on port 3040, for registered users

### Running

The following commands need to be run from the root of the Nym git project.

To build the entire dockerized environment, run the following command:

```
PERSONAL_TOKEN=[network explorer token] docker-compose build
```

or build each service separately, as changes are made to their relevant source code.
**Note** network-explorer build time is currently very high, so building it more than once is not advisable. 

To start the dockerized environment, run the following command:

```
METEOR_SETTINGS=$(cat docker/block_explorer/settings.json) docker-compose up -d --scale=secondary_validator=3
```

**Note**: The `secondary_validator=3` can take any other number as value, depending on the desired setup.

The web wallet interface will become available at `localhost:3000`.

The mnemonic needed to connect to the admin user, which also has a pre-added number of tokens, can be obtained by running:

```
docker logs nym_mnemonic_echo_1
```

To stop the dockerized environment, run:

```
docker-compose down
```
