use axum::error_handling::HandleErrorLayer;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    handler::Handler,
    response::AppendHeaders,
    routing::{get, post},
    BoxError, Json, Router,
};
use axum_extra::headers::{self, authorization::Bearer};
use axum_extra::TypedHeader;
use governor::clock::QuantaInstant;
use governor::middleware::NoOpMiddleware;
use hyper::header::CACHE_CONTROL;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::{governor::GovernorConfig, GovernorLayer};
use tower_http::auth::add_authorization::AddAuthorizationLayer;

use crate::error::{Error, ErrorMessage};
use crate::models::paste::{CreatePaste, Paste, UpdatePaste};
use crate::server::AppState;

async fn create_paste_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<CreatePaste>,
) -> Result<Json<()>, Error> {
    Paste::create(&app_state.pool, payload).await?;
    Ok(Json(()))
}

async fn view_paste_handler(
    Path(id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<impl IntoResponse, Error> {
    let paste = Paste::view(&app_state.pool, id).await?;
    if paste.expiry_views.is_none() {
        let now = SystemTime::now();
        let max_age = match paste.expiry_time {
            Some(expires) => {
                let max_age = expires - now.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
                format!("public, max-age={}", max_age)
            }
            None => "public, max-age=3600".to_string(),
        };

        return Ok((AppendHeaders([(CACHE_CONTROL, max_age)]), Json(paste)));
    }
    Ok((
        AppendHeaders([(CACHE_CONTROL, "no-cache".to_string())]),
        Json(paste),
    ))
}

async fn update_paste_handler(
    TypedHeader(_auth_header): TypedHeader<headers::Authorization<Bearer>>,
    Path(id): Path<String>,
    State(app_state): State<AppState>,
    Json(payload): Json<UpdatePaste>,
) -> Result<Json<()>, Error> {
    Paste::update(&app_state.pool, id, payload).await?;
    Ok(Json(()))
}

async fn delete_paste_handler(
    TypedHeader(_auth_header): TypedHeader<headers::Authorization<Bearer>>,
    Path(id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<Json<()>, Error> {
    Paste::delete(&app_state.pool, id).await?;
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
                .put(update_paste_handler.layer(AddAuthorizationLayer::bearer(&admin_token)))
                .delete(delete_paste_handler.layer(AddAuthorizationLayer::bearer(&admin_token))),
        )
}
