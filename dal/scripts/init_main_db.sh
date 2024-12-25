#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v psql)" ]; then
    echo >&2 "Error: psql is not installed."
    exit 1
fi

if ! [ -x "$(command -v sqlx)" ]; then
    echo >&2 "Error: sqlx is not installed."
    echo >&2 "Use:"
    echo >&2 " cargo install --version=0.5.7 sqlx-cli --no-default-features --features postgres"
    echo >&2 "to install it."
    exit 1
fi

# docker network create bitvm-bridge-db-sync
NETWORK_NAME="bitvm-bridge-db-sync"
if ! docker network inspect $NETWORK_NAME >/dev/null 2>&1; then
    echo "Network $NETWORK_NAME does not exist. Creating..."
    docker network create $NETWORK_NAME
else
    echo "Network $NETWORK_NAME already exists. Doing nothing."
fi

# Check if a custom user has been set, otherwise default to 'postgres'
DB_USER=${POSTGRES_USER:=admin}
# Check if a custom password has been set, otherwise default to 'password'
DB_PASSWORD="${POSTGRES_PASSWORD:=admin123}"
# Check if a custom database name has been set, otherwise default to 'bitvm-bridge'
DB_NAME="${POSTGRES_DB:=bitvm-bridge}"
# Check if a custom port has been set, otherwise default to '7432'
DB_PORT="${POSTGRES_PORT:=7432}"
# Check if a custom repl password has been set, otherwise default to 'repl123'
DB_REPL_PASSWORD="${POSTGRES_REPLICA_PASSWORD:=repl123}"
# --database-url postgres://admin:admin123@localhost:7432/bitvm-bridge
if [[ -z "${SKIP_DOCKER}" ]]
then
    # rm -rf ./scripts/data-backup
    docker run \
        --mount type=bind,source="$(pwd)"/scripts/main_postgresql.conf,target=/etc/postgresql/postgresql.conf \
        --mount type=bind,source="$(pwd)"/scripts/main_pg_hba.conf,target=/etc/postgresql/pg_hba.conf \
        -v ./scripts/bitvm_bridge_pgdata:/var/lib/postgresql/data \
        -v ./scripts/archivelog:/archivelog \
        -v ./scripts/data-backup:/tmp/postgresslave \
        --net $NETWORK_NAME \
        --name bitvm_bridge_main \
        -e POSTGRES_USER=${DB_USER} \
        -e POSTGRES_PASSWORD=${DB_PASSWORD} \
        -e POSTGRES_DB=${DB_NAME} \
        -p "${DB_PORT}":5432 \
        -d postgres \
        postgres -c config_file=/etc/postgresql/postgresql.conf
fi

export PGPASSWORD="${DB_PASSWORD}"
until psql -h "localhost" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
    >&2 echo "Postgres is still unavailable - sleeping"
    sleep 1
done

>&2 echo "Postgres is up and running on port ${DB_PORT}!"

export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}
sqlx database create
sqlx migrate run

>&2 echo "Postgres has been migrated"

psql -h "localhost" -U "${DB_USER}" -p "${DB_PORT}" -d "${DB_NAME}" -c "DROP ROLE IF EXISTS repl;"
psql -h "localhost" -U "${DB_USER}" -p "${DB_PORT}" -d "${DB_NAME}" -c "CREATE ROLE repl REPLICATION LOGIN ENCRYPTED PASSWORD '${DB_REPL_PASSWORD}';"

>&2 echo "Replication role repl has been created"

docker exec bitvm_bridge_main sh -c "pg_basebackup -h bitvm_bridge_main -U repl -p 5432 -F p -X s -P -R -D /tmp/postgresslave"

>&2 echo "pg_basebackup finished, ready to go!"