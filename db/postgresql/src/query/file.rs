use anyhow::Result;
use sqlx::{Executor, Pool, Postgres};
use uuid::Uuid;

use crate::{db::PostgresDb, model::file::FileModel};

const INSERT: &str = "INSERT INTO \"files\" (\"id\", \"created_by\", \"created_at\", \"updated_at\", \"bucket_id\", \"file_name\", \"content_type\", \"size\") VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";
const SELECT: &str = "SELECT \"id\", \"created_by\", \"created_at\", \"updated_at\", \"bucket_id\", \"file_name\", \"content_type\", \"size\" FROM \"files\" WHERE \"id\" = $1";
const SELECT_MANY_BY_BUCKET_ID: &str = "SELECT \"id\", \"created_by\", \"created_at\", \"updated_at\", \"bucket_id\", \"file_name\", \"content_type\", \"size\" FROM \"files\" WHERE \"bucket_id\" = $1";
const COUNT_MANY_BY_BUCKET_ID: &str = "SELECT COUNT(1) FROM \"files\" WHERE \"bucket_id\" = $1";
const SELECT_MANY_BY_CREATED_BY_AND_BUCKET_ID: &str = "SELECT \"id\", \"created_by\", \"created_at\", \"updated_at\", \"bucket_id\", \"file_name\", \"content_type\", \"size\" FROM \"files\" WHERE \"created_by\" = $1 AND \"bucket_id\" = $2";
const COUNT_MANY_BY_CREATED_BY_AND_BUCKET_ID: &str =
    "SELECT COUNT(1) FROM \"files\" WHERE \"created_by\" = $1 AND \"bucket_id\" = $2";
const UPDATE: &str = "UPDATE \"files\" SET \"created_by\" = $1, \"updated_at\" = $2, \"file_name\" = $3 WHERE \"id\" = $4";
const DELETE: &str = "DELETE FROM \"files\" WHERE \"id\" = $1";

pub async fn init(pool: &Pool<Postgres>) {
    hb_log::info(Some("🔧"), "PostgreSQL: Setting up files table");

    pool.execute("CREATE TABLE IF NOT EXISTS \"files\" (\"id\" uuid, \"created_by\" uuid, \"created_at\" timestamptz, \"updated_at\" timestamptz, \"bucket_id\" uuid, \"file_name\" text, \"content_type\" text, \"size\" bigint, PRIMARY KEY (\"id\"))").await.unwrap();

    pool.prepare(INSERT).await.unwrap();
    pool.prepare(SELECT).await.unwrap();
    pool.prepare(SELECT_MANY_BY_BUCKET_ID).await.unwrap();
    pool.prepare(SELECT_MANY_BY_CREATED_BY_AND_BUCKET_ID)
        .await
        .unwrap();
    pool.prepare(UPDATE).await.unwrap();
    pool.prepare(DELETE).await.unwrap();
}

impl PostgresDb {
    pub async fn insert_file(&self, value: &FileModel) -> Result<()> {
        self.execute(
            sqlx::query(INSERT)
                .bind(value.id())
                .bind(value.created_by())
                .bind(value.created_at())
                .bind(value.updated_at())
                .bind(value.bucket_id())
                .bind(value.file_name())
                .bind(value.content_type())
                .bind(value.size()),
        )
        .await?;
        Ok(())
    }

    pub async fn select_file(&self, id: &Uuid) -> Result<FileModel> {
        Ok(self.fetch_one(sqlx::query_as(SELECT).bind(id)).await?)
    }

    pub async fn select_many_files_by_bucket_id(
        &self,
        bucket_id: &Uuid,
        after_id: &Option<Uuid>,
        limit: &Option<i32>,
    ) -> Result<Vec<FileModel>> {
        let mut sql = SELECT_MANY_BY_BUCKET_ID.to_owned();
        let mut count_values = 1;
        if after_id.is_some() {
            count_values += 1;
            sql += &format!(" AND \"id\" < ${count_values}");
        }
        sql += " ORDER BY \"id\" DESC";
        if limit.is_some() {
            count_values += 1;
            sql += &format!(" LIMIT ${count_values}");
        }

        let mut query = sqlx::query_as(&sql).bind(bucket_id);
        if let Some(after_id) = after_id {
            query = query.bind(after_id);
        }
        if let Some(limit) = limit {
            query = query.bind(limit);
        }

        Ok(self.fetch_all(query).await?)
    }

    pub async fn count_many_files_by_bucket_id(
        &self,
        bucket_id: &Uuid,
        after_id: &Option<Uuid>,
    ) -> Result<i64> {
        let mut sql = COUNT_MANY_BY_BUCKET_ID.to_owned();
        let mut count_values = 1;
        if after_id.is_some() {
            count_values += 1;
            sql += &format!(" AND \"id\" < ${count_values}");
        }

        let mut query = sqlx::query_as(&sql).bind(bucket_id);
        if let Some(after_id) = after_id {
            query = query.bind(after_id);
        }

        Ok(self.fetch_one::<(i64,)>(query).await?.0)
    }

    pub async fn select_many_files_by_created_by_and_bucket_id(
        &self,
        created_by: &Uuid,
        bucket_id: &Uuid,
        after_id: &Option<Uuid>,
        limit: &Option<i32>,
    ) -> Result<Vec<FileModel>> {
        let mut sql = SELECT_MANY_BY_CREATED_BY_AND_BUCKET_ID.to_owned();
        let mut count_values = 2;
        if after_id.is_some() {
            count_values += 1;
            sql += &format!(" AND \"id\" < ${count_values}");
        }
        sql += " ORDER BY \"id\" DESC";
        if limit.is_some() {
            count_values += 1;
            sql += &format!(" LIMIT ${count_values}");
        }

        let mut query = sqlx::query_as(&sql).bind(created_by).bind(bucket_id);
        if let Some(after_id) = after_id {
            query = query.bind(after_id);
        }
        if let Some(limit) = limit {
            query = query.bind(limit);
        }

        Ok(self.fetch_all(query).await?)
    }

    pub async fn count_many_files_by_created_by_and_bucket_id(
        &self,
        created_by: &Uuid,
        bucket_id: &Uuid,
        after_id: &Option<Uuid>,
    ) -> Result<i64> {
        let mut sql = COUNT_MANY_BY_CREATED_BY_AND_BUCKET_ID.to_owned();
        let mut count_values = 2;
        if after_id.is_some() {
            count_values += 1;
            sql += &format!(" AND \"id\" < ${count_values}");
        }

        let mut query = sqlx::query_as(&sql).bind(created_by).bind(bucket_id);
        if let Some(after_id) = after_id {
            query = query.bind(after_id);
        }

        Ok(self.fetch_one::<(i64,)>(query).await?.0)
    }

    pub async fn update_file(&self, value: &FileModel) -> Result<()> {
        self.execute(
            sqlx::query(UPDATE)
                .bind(value.created_by())
                .bind(value.updated_at())
                .bind(value.file_name())
                .bind(value.id()),
        )
        .await?;
        Ok(())
    }

    pub async fn delete_file(&self, id: &Uuid) -> Result<()> {
        self.execute(sqlx::query(DELETE).bind(id)).await?;
        Ok(())
    }
}
