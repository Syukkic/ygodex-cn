use anyhow::{self, Result};
use chrono::Utc;
use db::init_db;
use helpers::{download_and_insert_cards, fetch_and_update_id, get_md5};
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
        download_and_insert_cards(&pg_pool).await?;
        // Insert first, update later ?
        fetch_and_update_id(&pg_pool).await?;
        UpdateChecker::update_record(&pg_pool, &remote_md5, Utc::now()).await?;
        println!("Update completed.")
    } else {
        println!("No update needed")
    }

    Ok(())
}
