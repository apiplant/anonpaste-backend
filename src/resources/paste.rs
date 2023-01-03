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
use serde::{Deserialize, Serialize};
use sqlx::Connection;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceBuilder;
use tower_governor::key_extractor::PeerIpKeyExtractor;
use tower_governor::{errors::display_error, governor::GovernorConfig, GovernorLayer};
use tower_http::auth::RequireAuthorizationLayer;

use crate::error::Error;
use crate::server::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatePaste {
    id: String,
    content: String,
    expiry_time: Option<i64>,
    expiry_views: Option<i64>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdatePaste {
    content: String,
    expiry_time: Option<i64>,
    expiry_views: Option<i64>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Paste {
    id: String,
    content: String,
    expiry_time: Option<i64>,
    expiry_views: Option<i64>,
}

async fn create_paste_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<CreatePaste>,
) -> Result<(), Error> {
    let mut conn = app_state.pool.acquire().await?;

    sqlx::query!(
        "INSERT INTO paste ( id, content, expiry_time, expiry_views )
            VALUES ( ?1, ?2, ?3, ?4)",
        payload.id,
        payload.content,
        payload.expiry_time,
        payload.expiry_views
    )
    .execute(&mut conn)
    .await?;

    Ok(())
}

async fn view_paste_handler(
    Path(id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<Json<Paste>, Error> {
    let mut conn = app_state.pool.acquire().await?;

    let paste = conn
        .transaction::<_, _, sqlx::error::Error>(|conn| {
            Box::pin(async move {
                let paste = sqlx::query_as!(
                    Paste,
                    "SELECT id, 
                            content, 
                            expiry_time, 
                            expiry_views 
                        FROM paste WHERE id = ?",
                    id,
                )
                .fetch_one(&mut *conn)
                .await?;

                if let Some(views) = paste.expiry_views {
                    if views > 0_i64 {
                        sqlx::query!(
                            "UPDATE paste
                            SET
                                expiry_views = MAX(0, expiry_views-1)
                            WHERE id = ?",
                            id,
                        )
                        .execute(conn)
                        .await?;
                    }
                }

                Ok(paste)
            })
        })
        .await?;

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if let Some(expiry_time) = paste.expiry_time {
        if expiry_time < time.try_into().unwrap() {
            return Err(Error::NotFound);
        }
    }
    if let Some(expiry_views) = paste.expiry_views {
        if expiry_views == 0 {
            return Err(Error::NotFound);
        }
    }

    Ok(Json(paste))
}

async fn update_paste_handler(
    TypedHeader(_auth_header): TypedHeader<headers::Authorization<Bearer>>,
    Path(id): Path<String>,
    State(app_state): State<AppState>,
    Json(payload): Json<UpdatePaste>,
) -> Result<(), Error> {
    let mut conn = app_state.pool.acquire().await?;

    sqlx::query!(
        "UPDATE paste
            SET
                content = ?1,
                expiry_time = ?2,
                expiry_views = ?3
            WHERE id = ?4",
        payload.content,
        payload.expiry_time,
        payload.expiry_views,
        id,
    )
    .execute(&mut conn)
    .await?;

    Ok(())
}

async fn delete_paste_handler(
    TypedHeader(_auth_header): TypedHeader<headers::Authorization<Bearer>>,
    Path(id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<(), Error> {
    let mut conn = app_state.pool.acquire().await?;

    sqlx::query!("DELETE FROM paste WHERE id = ?", id,)
        .execute(&mut conn)
        .await?;

    Ok(())
}

pub fn paste_routes(
    admin_token: &String,
    governor_config: Box<Rc<GovernorConfig<PeerIpKeyExtractor, NoOpMiddleware<QuantaInstant>>>>,
) -> Router<AppState> {
    Router::new()
        .route(
            "/api/paste",
            post(create_paste_handler).layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(|e: BoxError| async move {
                        display_error(e)
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
