use axum::error_handling::HandleErrorLayer;
use axum::{
    extract::{Path, State, TypedHeader},
    handler::Handler,
    headers::{self, authorization::Bearer},
    routing::{get, post},
    BoxError, Json, Router,
};
use governor::clock::QuantaInstant;
use governor::middleware::NoOpMiddleware;
use std::rc::Rc;
use tower::ServiceBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::{governor::GovernorConfig, GovernorLayer};
use tower_http::auth::RequireAuthorizationLayer;

use crate::error::{Error, ErrorMessage};
use crate::models::paste::{CreatePaste, Paste, UpdatePaste};
use crate::server::AppState;

async fn create_paste_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<CreatePaste>,
) -> Result<Json<()>, Error> {
    let mut conn = app_state.pool.acquire().await?;
    Paste::create(&mut conn, payload).await?;
    Ok(Json(()))
}

async fn view_paste_handler(
    Path(id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<Json<Paste>, Error> {
    let mut conn = app_state.pool.acquire().await?;
    let paste = Paste::view(&mut conn, id).await?;
    Ok(Json(paste))
}

async fn update_paste_handler(
    TypedHeader(_auth_header): TypedHeader<headers::Authorization<Bearer>>,
    Path(id): Path<String>,
    State(app_state): State<AppState>,
    Json(payload): Json<UpdatePaste>,
) -> Result<Json<()>, Error> {
    let mut conn = app_state.pool.acquire().await?;
    Paste::update(&mut conn, id, payload).await?;
    Ok(Json(()))
}

async fn delete_paste_handler(
    TypedHeader(_auth_header): TypedHeader<headers::Authorization<Bearer>>,
    Path(id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<Json<()>, Error> {
    let mut conn = app_state.pool.acquire().await?;
    Paste::delete(&mut conn, id).await?;
    Ok(Json(()))
}

pub fn paste_routes(
    admin_token: &String,
    governor_config: Box<Rc<GovernorConfig<SmartIpKeyExtractor, NoOpMiddleware<QuantaInstant>>>>,
) -> Router<AppState> {
    Router::new()
        .route(
            "/api/paste",
            post(create_paste_handler).layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(|e: BoxError| async move {
                        Json(ErrorMessage { msg: e.to_string() })
                    }))
                    .layer(GovernorLayer {
                        config: Box::leak(governor_config),
                    }),
            ),
        )
        .route(
            "/api/paste/:id",
            get(view_paste_handler)
                .put(update_paste_handler.layer(RequireAuthorizationLayer::bearer(&admin_token)))
                .delete(
                    delete_paste_handler.layer(RequireAuthorizationLayer::bearer(&admin_token)),
                ),
        )
}
