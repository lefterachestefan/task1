use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::env;

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct Listing {
    id: String,
    // title: String,
    // description: String,
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
    max_clusters: Option<i32>,
}

#[derive(Serialize)]
struct ClusterPoint {
    lat: f64,
    lon: f64,
    count: i64,
}

impl ListingFilters {
    fn is_search(&self) -> bool {
        self.min_rooms.is_some()
            || self.max_rooms.is_some()
            || self.min_price.is_some()
            || self.max_price.is_some()
            || self.listing_type.is_some()
            || self.min_area.is_some()
            || self.max_area.is_some()
            || self.min_floor.is_some()
            || self.max_floor.is_some()
            || self.tags.is_some()
            || self.min_lat.is_some()
            || self.max_lat.is_some()
            || self.min_lon.is_some()
            || self.max_lon.is_some()
    }

    fn apply_filters<'a>(&'a self, query_builder: &mut sqlx::QueryBuilder<'a, Postgres>) {
        if let Some(min_rooms) = self.min_rooms {
            query_builder.push(" AND rooms >= ").push_bind(min_rooms);
        }
        if let Some(max_rooms) = self.max_rooms {
            query_builder.push(" AND rooms <= ").push_bind(max_rooms);
        }
        if let Some(min_price) = self.min_price {
            query_builder.push(" AND price >= ").push_bind(min_price);
        }
        if let Some(max_price) = self.max_price {
            query_builder.push(" AND price <= ").push_bind(max_price);
        }
        if let Some(ref ltype) = self.listing_type {
            query_builder.push(" AND listing_type = ").push_bind(ltype);
        }
        if let Some(min_area) = self.min_area {
            query_builder.push(" AND area_sqm >= ").push_bind(min_area);
        }
        if let Some(max_area) = self.max_area {
            query_builder.push(" AND area_sqm <= ").push_bind(max_area);
        }
        if let Some(min_floor) = self.min_floor {
            query_builder.push(" AND floor >= ").push_bind(min_floor);
        }
        if let Some(max_floor) = self.max_floor {
            query_builder.push(" AND floor <= ").push_bind(max_floor);
        }
        if let Some(min_lat) = self.min_lat {
            query_builder.push(" AND lat >= ").push_bind(min_lat);
        }
        if let Some(max_lat) = self.max_lat {
            query_builder.push(" AND lat <= ").push_bind(max_lat);
        }
        if let Some(min_lon) = self.min_lon {
            query_builder.push(" AND lon >= ").push_bind(min_lon);
        }
        if let Some(max_lon) = self.max_lon {
            query_builder.push(" AND lon <= ").push_bind(max_lon);
        }

        if let Some(ref tags_str) = self.tags {
            let tags_vec: Vec<&str> = tags_str.split(',').collect();
            query_builder
                .push(" AND tags::jsonb @> ")
                .push_bind(serde_json::to_string(&tags_vec).unwrap_or_default())
                .push("::jsonb");
        }
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
async fn get_listings(
    filters: web::Query<ListingFilters>,
    data: web::Data<AppState>,
) -> impl Responder {
    let is_search = filters.is_search();
    let select_fields = if is_search {
        "id, rooms, area_sqm, price, listing_type, tags, lat, lon, floor"
    } else {
        "*"
    };

    let mut query_builder: sqlx::QueryBuilder<Postgres> =
        sqlx::QueryBuilder::new(format!("SELECT {} FROM listings WHERE 1=1 ", select_fields));

    filters.apply_filters(&mut query_builder);

    query_builder.push(" ORDER BY id ASC");

    let limit = filters.limit.unwrap_or(100).clamp(1, 500);
    query_builder.push(" LIMIT ").push_bind(limit);

    if is_search {
        let result = query_builder
            .build_query_as::<ListingSummary>()
            .fetch_all(&data.db)
            .await;
        match result {
            Ok(listings) => HttpResponse::Ok().json(listings),
            Err(e) => {
                eprintln!("Error fetching listings: {}", e);
                HttpResponse::InternalServerError().finish()
            }
        }
    } else {
        let result = query_builder
            .build_query_as::<Listing>()
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
}

#[get("/listings/clusters")]
async fn get_clusters(
    filters: web::Query<ListingFilters>,
    data: web::Data<AppState>,
) -> impl Responder {
    let min_lat = match filters.min_lat {
        Some(v) => v,
        None => return HttpResponse::BadRequest().body("min_lat required"),
    };
    let max_lat = match filters.max_lat {
        Some(v) => v,
        None => return HttpResponse::BadRequest().body("max_lat required"),
    };
    let min_lon = match filters.min_lon {
        Some(v) => v,
        None => return HttpResponse::BadRequest().body("min_lon required"),
    };
    let max_lon = match filters.max_lon {
        Some(v) => v,
        None => return HttpResponse::BadRequest().body("max_lon required"),
    };

    let max_clusters = filters.max_clusters.unwrap_or(10).clamp(1, 10);

    // We use a grid-based approach. For simplicity, we split into sqrt(max_clusters) buckets for each dimension.
    // For max_clusters=10, we can use a 3x3 grid (9 clusters) or 2x5.
    // Let's use a 3x3 grid if max_clusters is 9 or 10.
    let (lat_divs, lon_divs) = if max_clusters >= 9 {
        (3.0, 3.0)
    } else if max_clusters >= 8 {
        (4.0, 2.0)
    } else if max_clusters >= 6 {
        (3.0, 2.0)
    } else if max_clusters >= 4 {
        (2.0, 2.0)
    } else if max_clusters >= 2 {
        (2.0, 1.0)
    } else {
        (1.0, 1.0)
    };

    let lat_range = max_lat - min_lat;
    let lon_range = max_lon - min_lon;

    // To avoid division by zero
    let lat_range = if lat_range == 0.0 { 1.0 } else { lat_range };
    let lon_range = if lon_range == 0.0 { 1.0 } else { lon_range };

    let mut query_builder: sqlx::QueryBuilder<Postgres> = sqlx::QueryBuilder::new(
        "SELECT avg(lat) as lat, avg(lon) as lon, count(*) as count FROM listings WHERE 1=1 ",
    );

    filters.apply_filters(&mut query_builder);

    query_builder
        .push(" GROUP BY floor((lat - ")
        .push_bind(min_lat)
        .push(") / ")
        .push_bind(lat_range / lat_divs)
        .push("), ");
    query_builder
        .push(" floor((lon - ")
        .push_bind(min_lon)
        .push(") / ")
        .push_bind(lon_range / lon_divs)
        .push(")");

    let result: Result<Vec<(f64, f64, i64)>, _> =
        query_builder.build_query_as().fetch_all(&data.db).await;

    match result {
        Ok(clusters) => {
            let points: Vec<ClusterPoint> = clusters
                .into_iter()
                .map(|(lat, lon, count)| ClusterPoint { lat, lon, count })
                .collect();
            HttpResponse::Ok().json(points)
        }
        Err(e) => {
            eprintln!("Error fetching clusters: {}", e);
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
            .service(get_clusters)
            .service(get_listing_by_id)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test};

    #[actix_web::test]
    async fn test_get_clusters() {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&database_url)
            .expect("Failed to create pool");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState { db: pool.clone() }))
                .service(get_clusters),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/listings/clusters?min_lat=40&max_lat=50&min_lon=20&max_lon=30&max_clusters=5")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let clusters: Vec<ClusterPoint> = test::read_body_json(resp).await;
        assert!(clusters.len() <= 5);
        if !clusters.is_empty() {
            assert!(clusters[0].count >= 1);
        }
    }

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
                .service(health_check),
        )
        .await;

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
                .service(get_listings),
        )
        .await;

        let req = test::TestRequest::get().uri("/listings").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // Without search, it should return full Listings (including title)
        let listings: Vec<Listing> = test::read_body_json(resp).await;
        assert!(!listings.is_empty());
        assert!(!listings[0].title.is_empty());

        // Verify sort order
        for i in 0..listings.len() - 1 {
            assert!(listings[i].id <= listings[i + 1].id);
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
                .service(get_listings),
        )
        .await;

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
                .service(get_listing_by_id),
        )
        .await;

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
