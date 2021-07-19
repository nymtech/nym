## Build with Docker & Docker Compose

Currently you can build and run locally in a Docker containers the following components of the Nym Privacy Platform:

* One genesis validator
* Any number of secondary validators
* A contract uploader, that uploads the contract built from the local sources
* The web wallet application, accesible on port 3000

### Running

The following commands need to be run from the root of the Nym git project.

To start the dockerized environment, run the following command each time the local source code has changed:

```
docker-compose up --build -d --scale=secondary_validator=3
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
