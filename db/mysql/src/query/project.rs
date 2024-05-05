use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Executor, MySql, Pool};
use uuid::Uuid;

use crate::{db::MysqlDb, model::project::ProjectModel};

const INSERT: &str = "INSERT INTO `projects` (`id`, `created_at`, `updated_at`, `admin_id`, `name`) VALUES (?, ?, ?, ?, ?)";
const SELECT: &str = "SELECT `id`, `created_at`, `updated_at`, `admin_id`, `name` FROM `projects` WHERE `id` = ?";
const SELECT_MANY_BY_ADMIN_ID:  &str = "SELECT `id`, `created_at`, `updated_at`, `admin_id`, `name` FROM `projects` WHERE `admin_id` = ? ORDER BY `id` DESC";
const SELECT_MANY_FROM_UPDATED_AT_AND_AFTER_ID_WITH_LIMIT_ASC: &str = "SELECT `id`, `created_at`, `updated_at`, `admin_id`, `name` FROM `projects` WHERE `updated_at` > ? OR (`updated_at` = ? AND `id` > ?) ORDER BY `updated_at` ASC, `id` ASC LIMIT ?";
const UPDATE: &str = "UPDATE `projects` SET `updated_at` = ?, `admin_id` = ?, `name` = ? WHERE `id` = ?";
const DELETE: &str = "DELETE FROM `projects` WHERE `id` = ?";

pub async fn init(pool: &Pool<MySql>) {
    hb_log::info(Some("🔧"), "[MySQL] Setting up projects table");

    pool.execute("CREATE TABLE IF NOT EXISTS `projects` (`id` binary(16), `created_at` timestamp, `updated_at` timestamp, `admin_id` binary(16), `name` text, PRIMARY KEY (`id`))").await.unwrap();

    tokio::try_join!(
        pool.prepare(INSERT),
        pool.prepare(SELECT),
        pool.prepare(SELECT_MANY_BY_ADMIN_ID),
        pool.prepare(SELECT_MANY_FROM_UPDATED_AT_AND_AFTER_ID_WITH_LIMIT_ASC),
        pool.prepare(UPDATE),
        pool.prepare(DELETE),
    )
    .unwrap();
}

impl MysqlDb {
    pub async fn insert_project(&self, value: &ProjectModel) -> Result<()> {
        self.execute(
            sqlx::query(INSERT)
                .bind(value.id())
                .bind(value.created_at())
                .bind(value.updated_at())
                .bind(value.admin_id())
                .bind(value.name()),
        )
        .await?;
        Ok(())
    }

    pub async fn select_project(&self, id: &Uuid) -> Result<ProjectModel> {
        Ok(self.fetch_one(sqlx::query_as(SELECT).bind(id)).await?)
    }

    pub async fn select_many_projects_by_admin_id(
        &self,
        admin_id: &Uuid,
    ) -> Result<Vec<ProjectModel>> {
        Ok(self
            .fetch_all(sqlx::query_as(SELECT_MANY_BY_ADMIN_ID).bind(admin_id))
            .await?)
    }

    pub async fn select_many_projects_from_updated_at_and_after_id_with_limit_asc(
        &self,
        updated_at: &DateTime<Utc>,
        id: &Uuid,
        limit: &i32,
    ) -> Result<Vec<ProjectModel>> {
        Ok(self
            .fetch_all(
                sqlx::query_as(SELECT_MANY_FROM_UPDATED_AT_AND_AFTER_ID_WITH_LIMIT_ASC)
                    .bind(updated_at)
                    .bind(updated_at)
                    .bind(id)
                    .bind(limit),
            )
            .await?)
    }

    pub async fn update_project(&self, value: &ProjectModel) -> Result<()> {
        self.execute(
            sqlx::query(UPDATE)
                .bind(value.updated_at())
                .bind(value.admin_id())
                .bind(value.name())
                .bind(value.id()),
        )
        .await?;
        Ok(())
    }

    pub async fn delete_project(&self, id: &Uuid) -> Result<()> {
        self.execute(sqlx::query(DELETE).bind(id)).await?;
        Ok(())
    }
}
