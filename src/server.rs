use axum::{
    http::{HeaderValue, StatusCode},
    routing::get,
    Router,
};

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePool},
    Pool, Sqlite,
};
use std::{net::SocketAddr, rc::Rc, str::FromStr};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

use tower_governor::governor::GovernorConfigBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{mailer::Mailer, resources::paste::paste_routes, resources::report::report_routes};

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: Pool<Sqlite>,
    pub mailer: Mailer,
}

pub struct Config {
    pub db_url: String,
    pub frontend_origin: String,
    pub admin_token: String,
    pub sendgrid_api_key: String,
    pub email_from: String,
    pub email_name: String,
}

async fn health_handler() -> Result<String, (StatusCode, String)> {
    Ok("ok".to_string())
}

pub async fn run_server(
    Config {
        db_url,
        frontend_origin,
        admin_token,
        sendgrid_api_key,
        email_from,
        email_name,
    }: Config,
) -> anyhow::Result<()> {
    let governor_config = Box::new(Rc::new(
        GovernorConfigBuilder::default()
            .per_second(4)
            .burst_size(2)
            .finish()
            .unwrap(),
    ));

    let options = SqliteConnectOptions::from_str(&db_url)?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    let mailer = Mailer::new(sendgrid_api_key, email_from, email_name);
    let app_state = AppState { pool, mailer };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "anonpaste=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        .merge(paste_routes(&admin_token, governor_config.clone()))
        .merge(report_routes(&admin_token, governor_config))
        .route("/", get(health_handler))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(frontend_origin.parse::<HeaderValue>().unwrap())
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(CompressionLayer::new())
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], 8080));
    tracing::debug!("Starting server on  {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();

    Ok(())
}
