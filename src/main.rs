use std::collections::HashMap;
use std::io::Cursor;

use anyhow::{self, Context, Result};
use reqwest;
use serde::Deserialize;
use tokio;
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let ygo_card_zip_url = "https://ygocdb.com/api/v0/cards.zip";
    let response = reqwest::get(ygo_card_zip_url)
        .await
        .context("Failed to get cards.zip")?
        .bytes()
        .await
        .context("Can not ???")?;

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
            println!("{}", cards.len());
            break;
        }
    }
    Ok(())
}
