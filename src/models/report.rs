use crate::error::Error;
use crate::mailer::Mailer;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReport {
    pub links: Vec<String>,
    pub message: String,
    pub email: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportRow {
    pub links: String,
    pub message: String,
    pub email: String,
}
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub links: Vec<String>,
    pub message: String,
    pub email: String,
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

impl Report {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Report>, Error> {
        let reports = sqlx::query_as!(ReportRow, "SELECT links, message, email FROM report",)
            .fetch_all(pool)
            .await
            .map(|result| result.into_iter().map(|row| row.into()).collect())?;
        Ok(reports)
    }

    pub async fn create(
        pool: &SqlitePool,
        mailer: &Mailer,
        payload: CreateReport,
    ) -> Result<(), Error> {
        let mut conn = pool.acquire().await?;
        let links_txt = payload.links.join("\n");
        sqlx::query!(
            "INSERT INTO report ( links, message, email ) VALUES ( ?1, ?2, ?3 )",
            links_txt,
            payload.message,
            payload.email
        )
        .execute(&mut *conn)
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

        mailer.respond_to(&payload.email, &links).await?;

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: String) -> Result<(), Error> {
        let mut conn = pool.acquire().await?;
        sqlx::query!("DELETE FROM report WHERE id = ?", id)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}
