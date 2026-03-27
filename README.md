# Listing API - Task 1

A high-performance Rust web service built with **Actix Web** and **SQLx**, providing an API for real estate listings stored in a **PostgreSQL** database.

## Prerequisites

- **Rust**: Nightly toolchain (managed via the included `flake.nix`).
- **Nix**: For the development environment.
- **PostgreSQL**: Hosted locally.

## Project Structure

- `src/main.rs`: Actix Web server and API route implementations.
- `dump_1k.postgres.sql`: SQL schema and seed data (1,000 listings).
- `flake.nix`: Reproducible development environment with all required system dependencies (`openssl`, `pkg-config`, etc.).
- `.env`: Database connection configuration.

## Setup Instructions

### 1. Database Setup

Ensure PostgreSQL is running locally, then create the database and load the data:

```bash
# Create the user and database
sudo -u postgres psql -c "CREATE USER dazor WITH SUPERUSER;"
sudo -u postgres psql -c "CREATE DATABASE task1 OWNER dazor;"

# Load the schema and data
psql -d task1 -f dump_1k.postgres.sql
```

### 2. Environment Configuration

The project uses a Unix socket connection by default to simplify local authentication. Ensure your `.env` file looks like this:

```env
DATABASE_URL=postgres:///task1?host=/run/postgresql
```

### 3. Running the Server

Use `nix develop` to enter the shell with all dependencies, then run the application:

```bash
nix develop --command cargo run
```

The server will start at `http://127.0.0.1:8080`.

## API Endpoints

| Endpoint | Method | Description |
| :--- | :--- | :--- |
| `/health` | `GET` | Simple health check. |
| `/listings` | `GET` | Returns a list of the first 50 listings in JSON format. |
| `/listings/{id}` | `GET` | Returns details for a specific listing by its UUID. |

## Development

The project uses:
- **Actix Web**: For the asynchronous web framework.
- **SQLx**: For type-safe, asynchronous SQL queries using `rustls`.
- **Serde**: For JSON serialization/deserialization.
- **dotenvy**: For environment variable management.
