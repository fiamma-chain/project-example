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

until psql -h "localhost" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
    >&2 echo "Postgres is still unavailable - sleeping"
    sleep 1
done

>&2 echo "Postgres is up and running on port ${DB_PORT}!"

export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}
sqlx database create
sqlx migrate run