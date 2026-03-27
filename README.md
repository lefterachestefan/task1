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
| `/health` | `GET` | Returns `{"status":"ok"}`. |
| `/listings` | `GET` | Returns listing summaries (excluding title/description) sorted by ID. |
| `/listings/{id}` | `GET` | Returns full listing details for a single ID. |

### /listings Query Parameters

All parameters are optional and inclusive:
- `min_rooms`, `max_rooms`
- `min_price`, `max_price`
- `listing_type` (sale or rent)
- `min_area`, `max_area`
- `min_floor`, `max_floor`
- `tags` (comma-separated, AND semantics)
- `min_lat`, `max_lat`, `min_lon`, `max_lon` (bounding box)
- `limit` (1..500, default 100)

## Demo

### 1. Health Check
```bash
curl -s http://127.0.0.1:8080/health
```

### 2. Filtered Listings
```bash
# Get 3-room apartments for rent under 1500 with a quiet tag
curl -s "http://127.0.0.1:8080/listings?min_rooms=3&listing_type=rent&max_price=1500&tags=quiet" | jq .
```

### 3. Single Listing Detail
```bash
# Use an ID from the listings response
curl -s http://127.0.0.1:8080/listings/0009abff-42bd-be68-5417-8c7e61e6a2f9 | jq .
```

## Development

The project uses:
- **Actix Web**: For the asynchronous web framework.
- **SQLx**: For type-safe, asynchronous SQL queries using `rustls`.
- **Serde**: For JSON serialization/deserialization.
- **dotenvy**: For environment variable management.
