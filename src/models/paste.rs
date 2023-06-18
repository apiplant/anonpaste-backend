use crate::error::Error;
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolConnection;
use sqlx::{Connection, Sqlite};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaste {
    pub id: String,
    pub content: String,
    pub expiry_time: Option<i64>,
    pub expiry_views: Option<i64>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePaste {
    pub content: String,
    pub expiry_time: Option<i64>,
    pub expiry_views: Option<i64>,
}
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Paste {
    pub id: String,
    pub content: String,
    pub expiry_time: Option<i64>,
    pub expiry_views: Option<i64>,
}

impl Paste {
    pub async fn create(
        conn: &mut PoolConnection<Sqlite>,
        payload: CreatePaste,
    ) -> Result<(), Error> {
        sqlx::query!(
            "INSERT INTO paste ( id, content, expiry_time, expiry_views )
                VALUES ( ?1, ?2, ?3, ?4)",
            payload.id,
            payload.content,
            payload.expiry_time,
            payload.expiry_views
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn view(conn: &mut PoolConnection<Sqlite>, id: String) -> Result<Self, Error> {
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

        let time: i64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .try_into()
            .unwrap();

        if let Some(expiry_time) = paste.expiry_time {
            if expiry_time < time * 1000_i64 {
                return Err(Error::NotFound);
            }
        }
        if let Some(expiry_views) = paste.expiry_views {
            if expiry_views == 0 {
                return Err(Error::NotFound);
            }
        }
        Ok(paste)
    }

    pub async fn update(
        conn: &mut PoolConnection<Sqlite>,
        id: String,
        payload: UpdatePaste,
    ) -> Result<(), Error> {
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
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn delete(conn: &mut PoolConnection<Sqlite>, id: String) -> Result<(), Error> {
        sqlx::query!("DELETE FROM paste WHERE id = ?", id)
            .execute(conn)
            .await?;
        Ok(())
    }
}
