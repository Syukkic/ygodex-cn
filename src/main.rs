use anyhow::{self, Result};
use chrono::Utc;
use db::init_db;
use helpers::{
    download_cards_archiver, fetch_id_change_log, get_md5, insert_cards, update_cards_id,
};
use models::UpdateChecker;

mod db;
mod helpers;
mod models;

#[tokio::main]
async fn main() -> Result<()> {
    let pg_pool = init_db().await?;

    let cards_checksum_url = "https://ygocdb.com/api/v0/cards.zip.md5?callback=gu";
    let remote_md5 = get_md5(cards_checksum_url).await?;
    let local_md5 = UpdateChecker::get_last_updated_record(&pg_pool).await?;
    let needs_update = match local_md5 {
        Some(record) => record.md5_checksum != remote_md5,
        None => true,
    };

    if needs_update {
        let ygo_card_zip_url = "https://ygocdb.com/api/v0/cards.zip";
        let archive = download_cards_archiver(ygo_card_zip_url).await?;
        insert_cards(&pg_pool, archive).await?;
        // Insert first, update later ?
        let id_change_log_url = "https://ygocdb.com/api/v0/idChangelog.jsonp";
        let id_change_log = fetch_id_change_log(id_change_log_url).await?;
        update_cards_id(&pg_pool, id_change_log).await?;
        UpdateChecker::update_record(&pg_pool, &remote_md5, Utc::now()).await?;
        println!("Update completed.")
    } else {
        println!("No update needed")
    }

    Ok(())
}
