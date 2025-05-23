#!/bin/bash
set -e

export PGUSER="nym"
export PGPASSWORD="password1"
export PGPORT="5432"
export DB_NAME="nym_statistics_api"
export DATABASE_URL="postgres://${PGUSER}:${PGPASSWORD}@localhost:${PGPORT}/${DB_NAME}"

docker run --rm -it \
  --name ${DB_NAME} \
  -e POSTGRES_USER=${PGUSER} \
  -e POSTGRES_PASSWORD=${PGPASSWORD} \
  -e POSTGRES_DB=${DB_NAME} \
  -p ${PGPORT}:${PGPORT} \
  postgres
