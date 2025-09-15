#!/bin/bash
set -e

export PGUSER="nym"
export PGPASSWORD="password1"
export PGPORT="5432"
export DB_NAME="nym_statistics_api"
export DATABASE_URL="postgres://${PGUSER}:${PGPASSWORD}@localhost:${PGPORT}/${DB_NAME}"

cat <<EOF > .env
SQLX_OFFLINE=true
POSTGRES_USER=$PGUSER
POSTGRES_PASSWORD=$PGPASSWORD
PGPORT=$PGPORT
DB_NAME=$DB_NAME
DATABASE_URL=$DATABASE_URL
EOF

cat <<EOF > init_schema.sql
CREATE SCHEMA private_statistics_api;
EOF


docker run --rm -it \
  --name ${DB_NAME} \
  -e POSTGRES_USER=${PGUSER} \
  -e POSTGRES_PASSWORD=${PGPASSWORD} \
  -e POSTGRES_DB=${DB_NAME} \
  -v $(pwd)/init_schema.sql:/docker-entrypoint-initdb.d/init_schema.sql \
  -p ${PGPORT}:${PGPORT} \
  postgres

rm init_schema.sql


# sqlx migrate run
# cargo sqlx prepare