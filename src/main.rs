use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use dotenvy::dotenv;
use std::env;

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct Listing {
    id: String,
    title: String,
    description: String,
    rooms: i32,
    area_sqm: f64,
    price: f64,
    listing_type: String,
    tags: String, // Stored as TEXT in DB, contains JSON string
    lat: f64,
    lon: f64,
    floor: i32,
}

struct AppState {
    db: Pool<Postgres>,
}

#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[get("/listings")]
async fn get_listings(data: web::Data<AppState>) -> impl Responder {
    let result = sqlx::query_as::<_, Listing>("SELECT * FROM listings LIMIT 50")
        .fetch_all(&data.db)
        .await;

    match result {
        Ok(listings) => HttpResponse::Ok().json(listings),
        Err(e) => {
            eprintln!("Error fetching listings: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/listings/{id}")]
async fn get_listing_by_id(path: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let id = path.into_inner();
    let result = sqlx::query_as::<_, Listing>("SELECT * FROM listings WHERE id = $1")
        .bind(id)
        .fetch_optional(&data.db)
        .await;

    match result {
        Ok(Some(listing)) => HttpResponse::Ok().json(listing),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(e) => {
            eprintln!("Error fetching listing by id: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    println!("Server running at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState { db: pool.clone() }))
            .service(health_check)
            .service(get_listings)
            .service(get_listing_by_id)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
