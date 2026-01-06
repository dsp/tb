//! tb-web: Web interface for TigerBeetle.

use axum::routing::get;
use axum::Router;
use clap::Parser;
use std::net::SocketAddr;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;

mod api;
mod config;
mod error;
mod html;
mod routes;
mod state;
mod transport;

use config::Config;
use state::AppState;

/// Web interface for TigerBeetle.
#[derive(Parser, Debug)]
#[command(name = "tb-web")]
#[command(about = "Web interface for TigerBeetle", long_about = None)]
struct Args {
    /// Address to bind the web server.
    #[arg(long, default_value = "127.0.0.1:8080")]
    address: String,

    /// TigerBeetle cluster address.
    #[arg(long, default_value = "127.0.0.1:3000")]
    tb_address: String,

    /// TigerBeetle cluster ID.
    #[arg(long, default_value = "0")]
    cluster_id: u128,

    /// Log level (trace, debug, info, warn, error).
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&args.log_level)),
        )
        .init();

    // Parse addresses
    let address: SocketAddr = args.address.parse()?;
    let tb_address: SocketAddr = args.tb_address.parse()?;

    let config = Config {
        address,
        tb_address,
        cluster_id: args.cluster_id,
    };

    tracing::info!("Connecting to TigerBeetle at {}...", config.tb_address);

    // Create application state
    let state = AppState::new(config.clone()).await?;

    // Build router
    let app = Router::new()
        // API routes
        .route("/api/v1/accounts", get(routes::accounts::list_accounts))
        .route("/api/v1/accounts/{id}", get(routes::accounts::get_account))
        .route(
            "/api/v1/accounts/{id}/transfers",
            get(routes::accounts::get_account_transfers),
        )
        .route(
            "/api/v1/accounts/{id}/balances",
            get(routes::accounts::get_account_balances),
        )
        .route("/api/v1/transfers", get(routes::transfers::list_transfers))
        .route(
            "/api/v1/transfers/{id}",
            get(routes::transfers::get_transfer),
        )
        .route("/health", get(routes::health))
        // Frontend page routes (serve same content, HTMX handles detail loading)
        .route("/account/{id}", get(routes::frontend::serve_account_page))
        .route("/transfer/{id}", get(routes::frontend::serve_transfer_page))
        // Frontend fallback
        .fallback(routes::frontend::serve_frontend)
        // State
        .with_state(state)
        // Middleware
        .layer(CorsLayer::permissive())
        .layer(CompressionLayer::new());

    // Start server
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!("tb-web listening on http://{}", address);

    axum::serve(listener, app).await?;

    Ok(())
}
