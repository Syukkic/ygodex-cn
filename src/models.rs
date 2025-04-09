use anyhow::{self, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
pub struct CardDescription {
    pub types: String,
    pub pdesc: String,
    pub desc: String,
}

#[derive(Debug, Deserialize)]
pub struct CardAttribute {
    pub ot: i32,
    pub setcode: i64,
    #[serde(rename = "type")]
    pub type_: i32,
    pub atk: i32,
    pub def: i32,
    pub level: i32,
    pub race: i32,
    pub attribute: i32,
}

#[derive(Debug, Deserialize)]
pub struct YGOCard {
    pub cid: i32,
    pub id: i32,
    pub cn_name: Option<String>,
    pub sc_name: Option<String>,
    pub md_name: Option<String>,
    pub nwbbs_n: Option<String>,
    pub cnocg_n: Option<String>,
    pub jp_ruby: Option<String>,
    pub jp_name: Option<String>,
    pub en_name: Option<String>,
    pub text: CardDescription,
    pub data: Option<CardAttribute>,
    #[serde(skip_deserializing)]
    #[allow(dead_code)]
    pub is_extra: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UpdateChecker {
    pub md5_checksum: String,
    #[allow(dead_code)]
    pub last_updated: Option<DateTime<Utc>>,
}

impl UpdateChecker {
    pub async fn get_last_updated_record(pool: &PgPool) -> Result<Option<Self>> {
        let result = sqlx::query_as!(
            UpdateChecker,
            r#"
            SELECT md5_checksum, last_updated
            FROM update_checker
            ORDER BY last_updated DESC
            LIMIT 1
            "#
        )
        .fetch_optional(pool)
        .await?;

        Ok(result)
    }

    pub async fn update_record(
        pool: &PgPool,
        checksum: &str,
        last_updated: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO update_checker (md5_checksum, last_updated) VALUES ($1, $2)"#,
            checksum,
            last_updated
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
