#!/bin/bash

# .env is generated in build.rs
source .env

# Launching a container in such a way that it's destroyed after you detach from the terminal:
docker compose up

# docker exec -it nym-data-observatory-pg /bin/bash
# psql -U youruser -d yourdb

echo "Tearing down containers to have a clean slate"
docker compose down -v
