use axum::error_handling::HandleErrorLayer;
use axum::{
    extract::{Path, State, TypedHeader},
    handler::Handler,
    headers::{self, authorization::Bearer},
    routing::{delete, get, post},
    BoxError, Json, Router,
};
use governor::clock::QuantaInstant;
use governor::middleware::NoOpMiddleware;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

use tower::ServiceBuilder;
use tower_governor::key_extractor::PeerIpKeyExtractor;
use tower_governor::{errors::display_error, governor::GovernorConfig, GovernorLayer};
use tower_http::auth::RequireAuthorizationLayer;

use crate::error::Error;
use crate::server::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateReport {
    links: Vec<String>,
    message: String,
    email: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportRow {
    links: String,
    message: String,
    email: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    links: Vec<String>,
    message: String,
    email: String,
}

impl Into<Report> for ReportRow {
    fn into(self) -> Report {
        Report {
            links: self.links.split("\n").map(|s| s.to_string()).collect(),
            message: self.message,
            email: self.email,
        }
    }
}

type ReportList = Vec<Report>;

async fn list_report_handler(State(app_state): State<AppState>) -> Result<Json<ReportList>, Error> {
    let mut conn = app_state.pool.acquire().await?;
    let reports = sqlx::query_as!(ReportRow, "SELECT links, message, email FROM report",)
        .fetch_all(&mut conn)
        .await
        .map(|result| result.into_iter().map(|row| row.into()).collect())?;

    Ok(Json(reports))
}

async fn create_report_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateReport>,
) -> Result<(), Error> {
    let mut conn = app_state.pool.acquire().await?;

    let links_txt = payload.links.join("\n");
    sqlx::query!(
        "INSERT INTO report ( links, message, email ) VALUES ( ?1, ?2, ?3 )",
        links_txt,
        payload.message,
        payload.email
    )
    .execute(&mut conn)
    .await?;

    let (_link_ids, links): (Vec<String>, Vec<String>) = payload
        .links
        .into_iter()
        .filter_map(|link| {
            link.split(|c| c == '/' || c == '#')
                .nth(4)
                .map(|id| (String::from(id), link.clone()))
        })
        .unzip();

    app_state.mailer.respond_to(&payload.email, &links).await?;

    Ok(())
}

async fn delete_report_handler(
    TypedHeader(_auth_header): TypedHeader<headers::Authorization<Bearer>>,
    Path(id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<(), Error> {
    let mut conn = app_state.pool.acquire().await?;

    sqlx::query!("DELETE FROM report WHERE id = ?", id,)
        .execute(&mut conn)
        .await?;

    Ok(())
}

pub fn report_routes(
    admin_token: &String,
    governor_config: Box<Rc<GovernorConfig<PeerIpKeyExtractor, NoOpMiddleware<QuantaInstant>>>>,
) -> Router<AppState> {
    Router::new()
        .route(
            "/api/report/:id",
            delete(delete_report_handler).layer(RequireAuthorizationLayer::bearer(&admin_token)),
        )
        .route(
            "/api/report",
            get(list_report_handler.layer(RequireAuthorizationLayer::bearer(&admin_token))),
        )
        .route(
            "/api/report",
            post(create_report_handler).layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(|e: BoxError| async move {
                        display_error(e)
                    }))
                    .layer(GovernorLayer {
                        config: Box::leak(governor_config),
                    }),
            ),
        )
}
