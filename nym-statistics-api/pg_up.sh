#!/bin/bash
set -e

export PGUSER="nym"
export PGPASSWORD="password1"
export PGPORT="5432"
export DB_NAME="nym_statistics_api"
export DATABASE_URL="postgres://${PGUSER}:${PGPASSWORD}@localhost:${PGPORT}/${DB_NAME}"

cat <<EOF > .env
SQLX_OFFLINE=true
PGUSER=$PGUSER
PGPASSWORD=$PGPASSWORD
PGPORT=$PGPORT
DB_NAME=$DB_NAME
DATABASE_URL=$DATABASE_URL
EOF


docker run --rm -it \
  --name ${DB_NAME} \
  -e POSTGRES_USER=${PGUSER} \
  -e POSTGRES_PASSWORD=${PGPASSWORD} \
  -e POSTGRES_DB=${DB_NAME} \
  -p ${PGPORT}:${PGPORT} \
  postgres


# sqlx migrate run
# cargl sqlx prepare