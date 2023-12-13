use axum::error_handling::HandleErrorLayer;
use axum::{
    extract::{Path, State},
    handler::Handler,
    routing::{delete, get, post},
    BoxError, Json, Router,
};
use axum_extra::headers::{self, authorization::Bearer};
use axum_extra::TypedHeader;
use governor::clock::QuantaInstant;
use governor::middleware::NoOpMiddleware;
use std::rc::Rc;

use tower::ServiceBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::{governor::GovernorConfig, GovernorLayer};
use tower_http::auth::add_authorization::AddAuthorizationLayer;

use crate::error::{Error, ErrorMessage};
use crate::models::report::{CreateReport, Report};
use crate::server::AppState;

async fn list_report_handler(
    State(app_state): State<AppState>,
) -> Result<Json<Vec<Report>>, Error> {
    let reports = Report::list(&app_state.pool).await?;
    Ok(Json(reports))
}

async fn create_report_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateReport>,
) -> Result<Json<()>, Error> {
    Report::create(&app_state.pool, &app_state.mailer, payload).await?;
    Ok(Json(()))
}

async fn delete_report_handler(
    TypedHeader(_auth_header): TypedHeader<headers::Authorization<Bearer>>,
    Path(id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<Json<()>, Error> {
    Report::delete(&app_state.pool, id).await?;
    Ok(Json(()))
}

pub fn report_routes(
    admin_token: &String,
    governor_config: Box<Rc<GovernorConfig<SmartIpKeyExtractor, NoOpMiddleware<QuantaInstant>>>>,
) -> Router<AppState> {
    Router::new()
        .route(
            "/api/report/:id",
            delete(delete_report_handler).layer(AddAuthorizationLayer::bearer(&admin_token)),
        )
        .route(
            "/api/report",
            get(list_report_handler.layer(AddAuthorizationLayer::bearer(&admin_token))),
        )
        .route(
            "/api/report",
            post(create_report_handler).layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(|e: BoxError| async move {
                        Json(ErrorMessage { msg: e.to_string() })
                    }))
                    .layer(GovernorLayer {
                        config: Box::leak(governor_config),
                    }),
            ),
        )
}
