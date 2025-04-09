use std::collections::HashMap;
use std::io::Cursor;

use anyhow::{self, Context, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgPoolOptions};
use zip::ZipArchive;

#[derive(Debug, Deserialize)]
struct CardDescription {
    types: String,
    pdesc: String,
    desc: String,
}

#[derive(Debug, Deserialize)]
struct CardAttribute {
    ot: i32,
    setcode: i64,
    #[serde(rename = "type")]
    type_: i32,
    atk: i32,
    def: i32,
    level: i32,
    race: i32,
    attribute: i32,
}

#[derive(Debug, Deserialize)]
struct YGOCard {
    cid: i32,
    id: i32,
    cn_name: Option<String>,
    sc_name: Option<String>,
    md_name: Option<String>,
    nwbbs_n: Option<String>,
    cnocg_n: Option<String>,
    jp_ruby: Option<String>,
    jp_name: Option<String>,
    en_name: Option<String>,
    text: CardDescription,
    data: Option<CardAttribute>,
    #[serde(skip_deserializing)]
    #[allow(dead_code)]
    is_extra: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct UpdateChecker {
    md5_checksum: String,
    last_updated: Option<DateTime<Utc>>,
}

impl UpdateChecker {
    async fn get_last_updated_record(pool: &PgPool) -> Result<Option<Self>> {
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

    async fn update_record(
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

fn is_extra(card: &YGOCard) -> bool {
    let t = &card.text.types;
    t.contains("融合") || t.contains("同调") || t.contains("超量") || t.contains("连接")
}

async fn get_md5(md5_url: &str) -> Result<String> {
    let response = reqwest::get(md5_url).await?.text().await?;
    let re = Regex::new(r#"([a-fA-F0-9]{32})"#)?;
    re.captures(&response)
        .and_then(|caps| caps.get(0))
        .map(|s| s.as_str().to_string())
        .context("MD5 checksum not found in response")
}

async fn insert_card(pool: &PgPool, card: &YGOCard) -> Result<(), sqlx::Error> {
    let text = &card.text;
    let data = card.data.as_ref();

    let mut tx = pool.begin().await?;

    sqlx::query!(
        r#"
        INSERT INTO ygo_cards (
            cid, id, cn_name, sc_name, md_name, nwbbs_n, cnocg_n,
            jp_ruby, jp_name, en_name, types, pdesc, "desc", ot,
            setcode, "type", atk, def, level, race, attribute,
            is_extra
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7,
            $8, $9, $10, $11, $12, $13,
            $14, $15, $16, $17, $18, $19, $20,
            $21, $22
        ) ON CONFLICT (cid) DO NOTHING
        "#,
        card.cid,
        card.id as i64,
        card.cn_name.as_deref(),
        card.sc_name.as_deref(),
        card.md_name.as_deref(),
        card.nwbbs_n.as_deref(),
        card.cnocg_n.as_deref(),
        card.jp_ruby.as_deref(),
        card.jp_name.as_deref(),
        card.en_name.as_deref(),
        text.types,
        text.pdesc,
        text.desc,
        data.map(|d| d.ot),
        data.map(|d| d.setcode),
        data.map(|d| d.type_),
        data.map(|d| d.atk),
        data.map(|d| d.def),
        data.map(|d| d.level),
        data.map(|d| d.race),
        data.map(|d| d.attribute),
        is_extra(card),
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set.");
    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .context("Failed to connect to database")?;

    let cards_checksum_url = "https://ygocdb.com/api/v0/cards.zip.md5?callback=gu";
    let remote_md5 = get_md5(cards_checksum_url).await?;
    let local_md5 = UpdateChecker::get_last_updated_record(&pg_pool).await?;
    let needs_update = match local_md5 {
        Some(record) => record.md5_checksum != remote_md5,
        None => true,
    };

    if needs_update {
        let ygo_card_zip_url = "https://ygocdb.com/api/v0/cards.zip";

        let response = reqwest::get(ygo_card_zip_url)
            .await
            .context("Failed to get cards.zip")?
            .bytes()
            .await
            .context("Failed to read cards.zip content")?;

        let cursor = Cursor::new(response);
        let mut archive = ZipArchive::new(cursor)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_owned();
            if name.ends_with(".json") {
                let mut content = String::new();
                use std::io::Read;
                file.read_to_string(&mut content)?;

                // Deserialize
                let cards: HashMap<String, YGOCard> = serde_json::from_str(&content)?;
                for card in cards.values() {
                    insert_card(&pg_pool, card).await?;
                }
                break;
            }
        }
        UpdateChecker::update_record(&pg_pool, &remote_md5, Utc::now()).await?;
        println!("Update completed.")
    } else {
        println!("No update needed")
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    #[tokio::test]
    async fn test_get_md5_success() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"/**/ typeof gu === 'function' && gu("bd6f1e3351eb85b16ec2fc2ac86b6be2")"#,
            ))
            .mount(&mock_server)
            .await;

        let url = &format!("{}/api/v0/cards.zip.md5?callback=gu", mock_server.uri());
        let result = get_md5(url).await.unwrap();
        let expected = "bd6f1e3351eb85b16ec2fc2ac86b6be2".to_string();

        assert_eq!(result, expected)
    }

    #[tokio::test]
    async fn test_get_md5_not_found() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("MD5 checksum not found in response"),
            )
            .mount(&mock_server)
            .await;

        let url = &format!("{}/api/v0/cards.zip.md5?callback=gu", mock_server.uri());
        let result = get_md5(url).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "MD5 checksum not found in response"
        )
    }
}
