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
    tags: String,
    lat: f64,
    lon: f64,
    floor: i32,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct ListingSummary {
    id: String,
    rooms: i32,
    area_sqm: f64,
    price: f64,
    listing_type: String,
    tags: String,
    lat: f64,
    lon: f64,
    floor: i32,
}

#[derive(Deserialize)]
struct ListingFilters {
    min_rooms: Option<i32>,
    max_rooms: Option<i32>,
    min_price: Option<f64>,
    max_price: Option<f64>,
    listing_type: Option<String>,
    min_area: Option<f64>,
    max_area: Option<f64>,
    min_floor: Option<i32>,
    max_floor: Option<i32>,
    tags: Option<String>, // Comma-separated
    min_lat: Option<f64>,
    max_lat: Option<f64>,
    min_lon: Option<f64>,
    max_lon: Option<f64>,
    limit: Option<i32>,
}

impl ListingFilters {
    fn is_search(&self) -> bool {
        self.min_rooms.is_some() || self.max_rooms.is_some() ||
        self.min_price.is_some() || self.max_price.is_some() ||
        self.listing_type.is_some() ||
        self.min_area.is_some() || self.max_area.is_some() ||
        self.min_floor.is_some() || self.max_floor.is_some() ||
        self.tags.is_some() ||
        self.min_lat.is_some() || self.max_lat.is_some() ||
        self.min_lon.is_some() || self.max_lon.is_some()
    }
}

struct AppState {
    db: Pool<Postgres>,
}

#[get("/health")]
async fn health_check(data: web::Data<AppState>) -> impl Responder {
    match sqlx::query("SELECT 1").execute(&data.db).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({"status": "ok"})),
        Err(e) => {
            eprintln!("Health check database error: {}", e);
            HttpResponse::ServiceUnavailable().json(serde_json::json!({"status": "error"}))
        }
    }
}

#[get("/listings")]
async fn get_listings(filters: web::Query<ListingFilters>, data: web::Data<AppState>) -> impl Responder {
    let is_search = filters.is_search();
    let select_fields = if is_search {
        "id, rooms, area_sqm, price, listing_type, tags, lat, lon, floor"
    } else {
        "*"
    };

    let mut query_builder: sqlx::QueryBuilder<Postgres> = sqlx::QueryBuilder::new(
        format!("SELECT {} FROM listings WHERE 1=1 ", select_fields)
    );

    if let Some(min_rooms) = filters.min_rooms {
        query_builder.push(" AND rooms >= ").push_bind(min_rooms);
    }
    if let Some(max_rooms) = filters.max_rooms {
        query_builder.push(" AND rooms <= ").push_bind(max_rooms);
    }
    if let Some(min_price) = filters.min_price {
        query_builder.push(" AND price >= ").push_bind(min_price);
    }
    if let Some(max_price) = filters.max_price {
        query_builder.push(" AND price <= ").push_bind(max_price);
    }
    if let Some(ref ltype) = filters.listing_type {
        query_builder.push(" AND listing_type = ").push_bind(ltype);
    }
    if let Some(min_area) = filters.min_area {
        query_builder.push(" AND area_sqm >= ").push_bind(min_area);
    }
    if let Some(max_area) = filters.max_area {
        query_builder.push(" AND area_sqm <= ").push_bind(max_area);
    }
    if let Some(min_floor) = filters.min_floor {
        query_builder.push(" AND floor >= ").push_bind(min_floor);
    }
    if let Some(max_floor) = filters.max_floor {
        query_builder.push(" AND floor <= ").push_bind(max_floor);
    }
    if let Some(min_lat) = filters.min_lat {
        query_builder.push(" AND lat >= ").push_bind(min_lat);
    }
    if let Some(max_lat) = filters.max_lat {
        query_builder.push(" AND lat <= ").push_bind(max_lat);
    }
    if let Some(min_lon) = filters.min_lon {
        query_builder.push(" AND lon >= ").push_bind(min_lon);
    }
    if let Some(max_lon) = filters.max_lon {
        query_builder.push(" AND lon <= ").push_bind(max_lon);
    }

    if let Some(ref tags_str) = filters.tags {
        let tags_vec: Vec<&str> = tags_str.split(',').collect();
        query_builder.push(" AND tags::jsonb @> ").push_bind(serde_json::to_string(&tags_vec).unwrap_or_default()).push("::jsonb");
    }

    query_builder.push(" ORDER BY id ASC");

    let limit = filters.limit.unwrap_or(100).clamp(1, 500);
    query_builder.push(" LIMIT ").push_bind(limit);

    if is_search {
        let result = query_builder.build_query_as::<ListingSummary>().fetch_all(&data.db).await;
        match result {
            Ok(listings) => HttpResponse::Ok().json(listings),
            Err(e) => {
                eprintln!("Error fetching listings: {}", e);
                HttpResponse::InternalServerError().finish()
            }
        }
    } else {
        let result = query_builder.build_query_as::<Listing>().fetch_all(&data.db).await;
        match result {
            Ok(listings) => HttpResponse::Ok().json(listings),
            Err(e) => {
                eprintln!("Error fetching listings: {}", e);
                HttpResponse::InternalServerError().finish()
            }
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
    
    // Use connect_lazy to avoid panicking if the DB is down at startup
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(&database_url)
        .expect("Failed to create lazy pool");

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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_health_check() {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&database_url)
            .expect("Failed to create pool");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState { db: pool.clone() }))
                .service(health_check)
        ).await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body, serde_json::json!({"status": "ok"}));
    }

    #[actix_web::test]
    async fn test_get_listings_integration() {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&database_url)
            .expect("Failed to create pool");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState { db: pool.clone() }))
                .service(get_listings)
        ).await;

        let req = test::TestRequest::get().uri("/listings").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // Without search, it should return full Listings (including title)
        let listings: Vec<Listing> = test::read_body_json(resp).await;
        assert!(!listings.is_empty());
        assert!(!listings[0].title.is_empty());
        
        // Verify sort order
        for i in 0..listings.len()-1 {
            assert!(listings[i].id <= listings[i+1].id);
        }
    }

    #[actix_web::test]
    async fn test_get_listings_with_filters() {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&database_url)
            .expect("Failed to create pool");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState { db: pool.clone() }))
                .service(get_listings)
        ).await;

        // Test filtering by rooms and price
        let req = test::TestRequest::get()
            .uri("/listings?min_rooms=3&max_price=2000")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let listings: Vec<ListingSummary> = test::read_body_json(resp).await;
        for listing in listings {
            assert!(listing.rooms >= 3);
            assert!(listing.price <= 2000.0);
        }

        // Test filtering by tags
        let req = test::TestRequest::get()
            .uri("/listings?tags=furnished,quiet")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let listings: Vec<ListingSummary> = test::read_body_json(resp).await;
        for listing in listings {
            assert!(listing.tags.contains("furnished"));
            assert!(listing.tags.contains("quiet"));
        }
    }

    #[actix_web::test]
    async fn test_get_listing_by_id_integration() {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&database_url)
            .expect("Failed to create pool");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState { db: pool.clone() }))
                .service(get_listings)
                .service(get_listing_by_id)
        ).await;

        // First, get all listings to find a valid ID
        let req = test::TestRequest::get().uri("/listings").to_request();
        let resp = test::call_service(&app, req).await;
        let listings: Vec<Listing> = test::read_body_json(resp).await;
        let test_id = &listings[0].id;

        // Now test the detail endpoint
        let req = test::TestRequest::get()
            .uri(&format!("/listings/{}", test_id))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let listing: Listing = test::read_body_json(resp).await;
        assert_eq!(listing.id, *test_id);
    }
}
