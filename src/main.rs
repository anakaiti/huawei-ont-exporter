use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use tracing::{debug, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use prometheus::{Encoder, TextEncoder};
use std::env;
use std::time::{Duration, Instant};
use tokio::time;

mod client;
mod metrics;
mod parser;

use client::OntClient;
use metrics::{
    update_metrics, HTTP_REQUESTS_ERRORS, HTTP_REQUESTS_TOTAL, SCRAPE_DURATION, SCRAPE_ERRORS,
    SCRAPES_TOTAL,
};

fn get_env_var(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| {
        eprintln!("Error: Environment variable {} must be set", name);
        std::process::exit(1);
    })
}

async fn metrics_handler() -> impl Responder {
    HTTP_REQUESTS_TOTAL.inc();

    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        error!("Failed to encode metrics: {}", e);
        HTTP_REQUESTS_ERRORS.inc();
        return HttpResponse::InternalServerError().body("Failed to encode metrics");
    }

    match String::from_utf8(buffer) {
        Ok(s) => HttpResponse::Ok().content_type("text/plain").body(s),
        Err(e) => {
            error!("Failed to convert metrics buffer to string: {}", e);
            HTTP_REQUESTS_ERRORS.inc();
            HttpResponse::InternalServerError().body("Failed to encode metrics")
        }
    }
}

async fn health_handler() -> impl Responder {
    HTTP_REQUESTS_TOTAL.inc();
    HttpResponse::Ok().body("OK")
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let ont_url = get_env_var("ONT_URL");
    let ont_user = get_env_var("ONT_USER");
    let ont_pass = get_env_var("ONT_PASS");
    let scrape_interval = env::var("SCRAPE_INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    info!("Starting ONT Metrics Scraper");
    info!("Target URL: {}", ont_url);
    info!("Scrape Interval: {}s", scrape_interval);

    // Spawn background scraping task
    let url = ont_url.clone();
    let user = ont_user.clone();
    let pass = ont_pass.clone();

    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(scrape_interval));
        loop {
            interval.tick().await;
            debug!("Scraping metrics...");

            SCRAPES_TOTAL.inc();
            let start = Instant::now();

            // Create a new client for each scrape to ensure fresh session state
            match OntClient::new(&url, &user, &pass) {
                Ok(client) => {
                    match client.scrape_metrics().await {
                        Ok(metrics) => {
                            let duration = start.elapsed().as_secs_f64();
                            SCRAPE_DURATION.observe(duration);
                            debug!("Scrape successful: {:?}", metrics);
                            update_metrics(&metrics);
                        }
                        Err(e) => {
                            SCRAPE_ERRORS.inc();
                            error!("Scrape failed: {:#}", e);
                        }
                    }
                }
                Err(e) => {
                    SCRAPE_ERRORS.inc();
                    error!("Failed to create ONT client: {}", e);
                }
            }
        }
    });

    info!("Starting HTTP server on 0.0.0.0:8000");
    HttpServer::new(|| {
        App::new()
            .route("/metrics", web::get().to(metrics_handler))
            .route("/health", web::get().to(health_handler))
    })
    .workers(2)
    .bind("0.0.0.0:8000")?
    .run()
    .await
}
