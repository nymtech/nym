# Nym-statistics-api

A simple API to collect and store statistics sent by nym-vpn-client. 

## Build instructions

The statistics API is backed by a PostgreSQL database so you'll need a PostgreSQL server running if you want to add migrations or add/modify SQL queries. I recommend https://postgresapp.com on MacOS, very easy to use. If you're on another OS, it's up to you.

Assuming your database is running at `postgresql://user:password@host:port/database_name` you'll likely need to run the following : 
```bash
DATABASE_URL="postgresql://user:password@host:port/database_name"

# if you don't have an existing datase
sqlx database create --database-url $DATABASE_URL
sqlx migrate run --database-url $DATABASE_URL

# reset it if you messed with migrations while developping
sqlx database reset --database-url $DATABASE_URL

# or just run new migrations
sqlx migrate run --database-url $DATABASE_URL

# then prepare queries for offline build mode
cargo sqlx prepare --database-url $DATABASE_URL
``` 

This should allow `cargo build` without having any postgreSQL server running.
Be sure to add the `.sqlx` directory to version control
