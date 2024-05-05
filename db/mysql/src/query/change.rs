use anyhow::Result;
use sqlx::{Executor, MySql, Pool};

use crate::{db::MysqlDb, model::change::ChangeModel};

const INSERT: &str = "INSERT INTO `changes` (`table`, `id`, `state`, `updated_at`, `change_id`) VALUES (?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE `id` = ?";
const UPSERT: &str = "INSERT INTO `changes` (`table`, `id`, `state`, `updated_at`, `change_id`) VALUES (?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE `state` = ?, `updated_at` = ?, `change_id` = ?";
const SELECT_LAST_BY_TABLE: &str = "SELECT `table`, `id`, `state`, `updated_at`, `change_id` FROM `changes` WHERE `table` = ? ORDER BY `updated_at` DESC, `change_id` DESC LIMIT 1";

pub async fn init(pool: &Pool<MySql>) {
    hb_log::info(Some("🔧"), "[MySQL] Setting up changes table");

    pool.execute("CREATE TABLE IF NOT EXISTS `changes` (`table` text, `id` binary(16), `state` text, `updated_at` timestamp, `change_id` binary(16), PRIMARY KEY (`table`, `id`))").await.unwrap();

    tokio::try_join!(
        pool.prepare(INSERT),
        pool.prepare(UPSERT),
        pool.prepare(SELECT_LAST_BY_TABLE),
    )
    .unwrap();
}

impl MysqlDb {
    pub async fn insert_change(&self, value: &ChangeModel) -> Result<()> {
        self.execute(
            sqlx::query(INSERT)
                .bind(value.table())
                .bind(value.id())
                .bind(value.state())
                .bind(value.updated_at())
                .bind(value.change_id())
                .bind(value.id()),
        )
        .await?;
        Ok(())
    }

    pub async fn upsert_change(&self, value: &ChangeModel) -> Result<()> {
        self.execute(
            sqlx::query(UPSERT)
                .bind(value.table())
                .bind(value.id())
                .bind(value.state())
                .bind(value.updated_at())
                .bind(value.change_id())
                .bind(value.state())
                .bind(value.updated_at())
                .bind(value.change_id()),
        )
        .await?;
        Ok(())
    }

    pub async fn select_last_change_by_table(&self, table: &str) -> Result<Option<ChangeModel>> {
        let data = self
            .fetch_one(sqlx::query_as(SELECT_LAST_BY_TABLE).bind(table))
            .await;
        match data {
            Ok(data) => Ok(Some(data)),
            Err(err) => {
                if matches!(err, sqlx::Error::RowNotFound) {
                    Ok(None)
                } else {
                    Err(err.into())
                }
            }
        }
    }
}
