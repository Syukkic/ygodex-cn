use std::collections::HashMap;
use std::io::Cursor;

use anyhow::{self, Context, Result};
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
    is_extra: bool,
}

fn is_extra(card: &YGOCard) -> bool {
    let t = &card.text.types;
    t.contains("融合") || t.contains("同调") || t.contains("超量") || t.contains("连接")
}

async fn insert_card(pool: &PgPool, card: &YGOCard) -> Result<(), sqlx::Error> {
    let text = &card.text;
    let data = card.data.as_ref();

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
    .execute(pool)
    .await?;

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
            println!("Insert {} cards", cards.len());
            break;
        }
    }

    Ok(())
}
