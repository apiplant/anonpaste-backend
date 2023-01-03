use anonpaste::server::{run_server, Config};
use anyhow::Context;

use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let db_url = env::var("DATABASE_URL").context("Please provide a DATABASE_URL")?;
    let frontend_origin =
        env::var("FRONTEND_ORIGIN").context("Please provide a FRONTEND_ORIGIN")?;
    let admin_token = env::var("ADMIN_TOKEN").context("Please provide an ADMIN_TOKEN")?;
    let sendgrid_api_key =
        env::var("SENDGRID_API_KEY").context("Please provide an SENDGRID_API_KEY")?;
    let email_from = env::var("EMAIL_FROM").context("Please provide an EMAIL_FROM")?;
    let email_name = env::var("EMAIL_NAME").context("Please provide an EMAIL_NAME")?;

    run_server(Config {
        db_url,
        frontend_origin,
        admin_token,
        sendgrid_api_key,
        email_from,
        email_name,
    })
    .await?;

    Ok(())
}
