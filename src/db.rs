use std::collections::HashMap;

use anyhow::{self, Context, Result};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{helpers::is_extra, models::YGOCard};

pub async fn init_db() -> Result<PgPool> {
    dotenvy::dotenv()?;
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set.");
    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .context("Failed to connect to database")?;

    Ok(pg_pool)
}

pub async fn insert_card(pool: &PgPool, card: &YGOCard) -> Result<(), sqlx::Error> {
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

pub async fn update_id_change_log(pool: &PgPool, changlog: HashMap<String, i64>) -> Result<()> {
    let mut tx = pool.begin().await?;
    for (old_id_string, new_id) in changlog {
        if let Ok(old_id) = old_id_string.parse::<i64>() {
            sqlx::query!(
                r#"UPDATE ygo_cards SET id = $1 WHERE id = $2"#,
                new_id,
                old_id
            )
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
