# Social Gaming Group 6 Backend

## Building
* Rust compiler (https://rustup.rs/)
* libpq (Postgres client library)

To build simply run `cargo build` or `cargo build --release` for a release build

### Database

Create a database and as a superuser execute:
```sql
create extension if not exist pgcrypto;
create extension if not exist postgis;
```

To install the postgis extension see https://postgis.net/

## Running

* Postgres (https://www.postgresql.org/) with pgcrypto and
* Postgis (https://postgis.net/) extensions

Check configuration in `.env` and run with `cargo run` or `cargo run --release` (this will also compile if there are no changes)

Or you could simply run the executable created by `cargo build` or `cargo build --release` in `target/{release,debug}/backend`
