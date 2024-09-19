#!/bin/bash

source .env

# Launching a container in such a way that it's destroyed after you detach from the terminal:
docker compose up --abort-on-container-exit --remove-orphans

# docker exec -it nym-data-observatory-pg /bin/bash
# psql -U youruser -d yourdb
