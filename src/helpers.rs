use std::{collections::HashMap, io::Cursor};

use anyhow::{self, Context, Result};
use regex::Regex;
use sqlx::PgPool;
use zip::ZipArchive;

use crate::{
    db::{insert_card, update_id_change_log},
    models::YGOCard,
};

pub fn is_extra(card: &YGOCard) -> bool {
    let t = &card.text.types;
    t.contains("融合") || t.contains("同调") || t.contains("超量") || t.contains("连接")
}

pub async fn get_md5(md5_url: &str) -> Result<String> {
    let response = reqwest::get(md5_url).await?.text().await?;
    let re = Regex::new(r#"([a-fA-F0-9]{32})"#)?;
    re.captures(&response)
        .and_then(|caps| caps.get(0))
        .map(|s| s.as_str().to_string())
        .context("MD5 checksum not found in response")
}

pub async fn download_and_insert_cards(pool: &PgPool) -> Result<()> {
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
                insert_card(pool, card).await?;
            }
            break;
        }
    }
    Ok(())
}

pub async fn fetch_and_update_id(pool: &PgPool) -> Result<()> {
    let id_change_log_url = "https://ygocdb.com/api/v0/idChangelog.jsonp";
    let response = reqwest::get(id_change_log_url).await?.text().await?;
    let changelog: HashMap<String, i64> =
        serde_json::from_str(&response).context("Failed to parse IdChangeLog")?;

    update_id_change_log(pool, changelog).await?;

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
