use anyhow::Result;
use chrono::{DateTime, Utc};
use hb_db_mysql::model::token::TokenModel as TokenMysqlModel;
use hb_db_postgresql::model::token::TokenModel as TokenPostgresModel;
use hb_db_scylladb::model::token::TokenModel as TokenScyllaModel;
use hb_db_sqlite::model::token::TokenModel as TokenSqliteModel;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use scylla::frame::value::CqlTimestamp as ScyllaCqlTimestamp;
use uuid::Uuid;

use crate::{bucket_rule::BucketRuleDao, collection_rule::CollectionRuleDao, util::conversion, Db};

pub struct TokenDao {
    id: Uuid,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    project_id: Uuid,
    admin_id: Uuid,
    token: String,
    allow_anonymous: bool,
    expired_at: Option<DateTime<Utc>>,
}

impl TokenDao {
    pub fn new(
        project_id: &Uuid,
        admin_id: &Uuid,
        token_length: &usize,
        allow_anonymous: &bool,
        expired_at: &Option<DateTime<Utc>>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            created_at: now,
            updated_at: now,
            project_id: *project_id,
            admin_id: *admin_id,
            token: thread_rng()
                .sample_iter(&Alphanumeric)
                .take(*token_length)
                .map(char::from)
                .collect(),
            allow_anonymous: *allow_anonymous,
            expired_at: *expired_at,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub fn updated_at(&self) -> &DateTime<Utc> {
        &self.updated_at
    }

    pub fn project_id(&self) -> &Uuid {
        &self.project_id
    }

    pub fn admin_id(&self) -> &Uuid {
        &self.admin_id
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn allow_anonymous(&self) -> &bool {
        &self.allow_anonymous
    }

    pub fn expired_at(&self) -> &Option<DateTime<Utc>> {
        &self.expired_at
    }

    pub fn set_allow_anonymous(&mut self, allow_anonymous: &bool) {
        self.allow_anonymous = *allow_anonymous;
    }

    pub fn set_expired_at(&mut self, expired_at: &Option<DateTime<Utc>>) {
        self.expired_at = *expired_at;
    }

    pub async fn is_allow_find_one_file(&self, db: &Db, bucket_id: &Uuid) -> bool {
        if let Ok(bucket_rule_data) =
            BucketRuleDao::db_select_by_token_id_and_bucket_id(db, &self.id, bucket_id).await
        {
            *bucket_rule_data.find_one()
        } else {
            false
        }
    }

    pub async fn is_allow_find_many_files(&self, db: &Db, bucket_id: &Uuid) -> bool {
        if let Ok(bucket_rule_data) =
            BucketRuleDao::db_select_by_token_id_and_bucket_id(db, &self.id, bucket_id).await
        {
            *bucket_rule_data.find_many()
        } else {
            false
        }
    }

    pub async fn is_allow_insert_file(&self, db: &Db, bucket_id: &Uuid) -> bool {
        if let Ok(bucket_rule_data) =
            BucketRuleDao::db_select_by_token_id_and_bucket_id(db, &self.id, bucket_id).await
        {
            *bucket_rule_data.insert_one()
        } else {
            false
        }
    }

    pub async fn is_allow_update_file(&self, db: &Db, bucket_id: &Uuid) -> bool {
        if let Ok(bucket_rule_data) =
            BucketRuleDao::db_select_by_token_id_and_bucket_id(db, &self.id, bucket_id).await
        {
            *bucket_rule_data.update_one()
        } else {
            false
        }
    }

    pub async fn is_allow_delete_file(&self, db: &Db, bucket_id: &Uuid) -> bool {
        if let Ok(bucket_rule_data) =
            BucketRuleDao::db_select_by_token_id_and_bucket_id(db, &self.id, bucket_id).await
        {
            *bucket_rule_data.update_one()
        } else {
            false
        }
    }

    pub async fn is_allow_find_one_record(&self, db: &Db, collection_id: &Uuid) -> bool {
        if let Ok(bucket_rule_data) =
            CollectionRuleDao::db_select_by_token_id_and_collection_id(db, &self.id, collection_id)
                .await
        {
            *bucket_rule_data.find_one()
        } else {
            false
        }
    }

    pub async fn is_allow_find_many_records(&self, db: &Db, collection_id: &Uuid) -> bool {
        if let Ok(bucket_rule_data) =
            CollectionRuleDao::db_select_by_token_id_and_collection_id(db, &self.id, collection_id)
                .await
        {
            *bucket_rule_data.find_many()
        } else {
            false
        }
    }

    pub async fn is_allow_insert_record(&self, db: &Db, collection_id: &Uuid) -> bool {
        if let Ok(bucket_rule_data) =
            CollectionRuleDao::db_select_by_token_id_and_collection_id(db, &self.id, collection_id)
                .await
        {
            *bucket_rule_data.insert_one()
        } else {
            false
        }
    }

    pub async fn is_allow_update_record(&self, db: &Db, collection_id: &Uuid) -> bool {
        if let Ok(bucket_rule_data) =
            CollectionRuleDao::db_select_by_token_id_and_collection_id(db, &self.id, collection_id)
                .await
        {
            *bucket_rule_data.update_one()
        } else {
            false
        }
    }

    pub async fn is_allow_delete_record(&self, db: &Db, collection_id: &Uuid) -> bool {
        if let Ok(bucket_rule_data) =
            CollectionRuleDao::db_select_by_token_id_and_collection_id(db, &self.id, collection_id)
                .await
        {
            *bucket_rule_data.delete_one()
        } else {
            false
        }
    }

    pub async fn db_insert(&self, db: &Db) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => db.insert_token(&self.to_scylladb_model()).await,
            Db::PostgresqlDb(db) => db.insert_token(&self.to_postgresdb_model()).await,
            Db::MysqlDb(db) => db.insert_token(&self.to_mysqldb_model()).await,
            Db::SqliteDb(db) => db.insert_token(&self.to_sqlitedb_model()).await,
        }
    }

    pub async fn db_select(db: &Db, id: &Uuid) -> Result<Self> {
        match db {
            Db::ScyllaDb(db) => Self::from_scylladb_model(&db.select_token(id).await?),
            Db::PostgresqlDb(db) => Ok(Self::from_postgresdb_model(&db.select_token(id).await?)),
            Db::MysqlDb(db) => Ok(Self::from_mysqldb_model(&db.select_token(id).await?)),
            Db::SqliteDb(db) => Ok(Self::from_sqlitedb_model(&db.select_token(id).await?)),
        }
    }

    pub async fn db_select_many_by_admin_id(db: &Db, admin_id: &Uuid) -> Result<Vec<Self>> {
        match db {
            Db::ScyllaDb(db) => {
                let mut tokens_data = Vec::new();
                for token in db.select_many_tokens_by_admin_id(admin_id).await? {
                    tokens_data.push(Self::from_scylladb_model(&token?)?);
                }
                Ok(tokens_data)
            }
            Db::PostgresqlDb(db) => Ok(db
                .select_many_tokens_by_admin_id(admin_id)
                .await?
                .iter()
                .map(|data| Self::from_postgresdb_model(data))
                .collect()),
            Db::MysqlDb(db) => Ok(db
                .select_many_tokens_by_admin_id(admin_id)
                .await?
                .iter()
                .map(|data| Self::from_mysqldb_model(data))
                .collect()),
            Db::SqliteDb(db) => Ok(db
                .select_many_tokens_by_admin_id(admin_id)
                .await?
                .iter()
                .map(|data| Self::from_sqlitedb_model(data))
                .collect()),
        }
    }

    pub async fn db_select_many_by_project_id(db: &Db, project_id: &Uuid) -> Result<Vec<Self>> {
        match db {
            Db::ScyllaDb(db) => {
                let mut tokens_data = Vec::new();
                for token in db.select_many_tokens_by_project_id(project_id).await? {
                    tokens_data.push(Self::from_scylladb_model(&token?)?);
                }
                Ok(tokens_data)
            }
            Db::PostgresqlDb(db) => Ok(db
                .select_many_tokens_by_project_id(project_id)
                .await?
                .iter()
                .map(|data| Self::from_postgresdb_model(data))
                .collect()),
            Db::MysqlDb(db) => Ok(db
                .select_many_tokens_by_project_id(project_id)
                .await?
                .iter()
                .map(|data| Self::from_mysqldb_model(data))
                .collect()),
            Db::SqliteDb(db) => Ok(db
                .select_many_tokens_by_project_id(project_id)
                .await?
                .iter()
                .map(|data| Self::from_sqlitedb_model(data))
                .collect()),
        }
    }

    pub async fn db_update(&mut self, db: &Db) -> Result<()> {
        self.updated_at = Utc::now();
        match db {
            Db::ScyllaDb(db) => db.update_token(&self.to_scylladb_model()).await,
            Db::PostgresqlDb(db) => db.update_token(&self.to_postgresdb_model()).await,
            Db::MysqlDb(db) => db.update_token(&self.to_mysqldb_model()).await,
            Db::SqliteDb(db) => db.update_token(&self.to_sqlitedb_model()).await,
        }
    }

    pub async fn db_delete(db: &Db, id: &Uuid) -> Result<()> {
        tokio::try_join!(
            CollectionRuleDao::db_delete_many_by_token_id(db, id),
            BucketRuleDao::db_delete_many_by_token_id(db, id)
        )?;

        match db {
            Db::ScyllaDb(db) => db.delete_token(id).await,
            Db::PostgresqlDb(db) => db.delete_token(id).await,
            Db::MysqlDb(db) => db.delete_token(id).await,
            Db::SqliteDb(db) => db.delete_token(id).await,
        }
    }

    fn from_scylladb_model(model: &TokenScyllaModel) -> Result<Self> {
        Ok(Self {
            id: *model.id(),
            created_at: conversion::scylla_cql_timestamp_to_datetime_utc(model.created_at())?,
            updated_at: conversion::scylla_cql_timestamp_to_datetime_utc(model.updated_at())?,
            project_id: *model.project_id(),
            admin_id: *model.admin_id(),
            token: model.token().to_owned(),
            allow_anonymous: *model.allow_anonymous(),
            expired_at: match &model.expired_at() {
                Some(expired_at) => Some(conversion::scylla_cql_timestamp_to_datetime_utc(
                    expired_at,
                )?),
                None => None,
            },
        })
    }

    fn to_scylladb_model(&self) -> TokenScyllaModel {
        TokenScyllaModel::new(
            &self.id,
            &ScyllaCqlTimestamp(self.created_at.timestamp_millis()),
            &ScyllaCqlTimestamp(self.updated_at.timestamp_millis()),
            &self.project_id,
            &self.admin_id,
            &self.token,
            &self.allow_anonymous,
            &match &self.expired_at {
                Some(expired_at) => Some(ScyllaCqlTimestamp(expired_at.timestamp_millis())),
                None => None,
            },
        )
    }

    fn from_postgresdb_model(model: &TokenPostgresModel) -> Self {
        Self {
            id: *model.id(),
            created_at: *model.created_at(),
            updated_at: *model.updated_at(),
            project_id: *model.project_id(),
            admin_id: *model.admin_id(),
            token: model.token().to_owned(),
            allow_anonymous: *model.allow_anonymous(),
            expired_at: *model.expired_at(),
        }
    }

    fn to_postgresdb_model(&self) -> TokenPostgresModel {
        TokenPostgresModel::new(
            &self.id,
            &self.created_at,
            &self.updated_at,
            &self.project_id,
            &self.admin_id,
            &self.token,
            &self.allow_anonymous,
            &self.expired_at,
        )
    }

    fn from_mysqldb_model(model: &TokenMysqlModel) -> Self {
        Self {
            id: *model.id(),
            created_at: *model.created_at(),
            updated_at: *model.updated_at(),
            project_id: *model.project_id(),
            admin_id: *model.admin_id(),
            token: model.token().to_owned(),
            allow_anonymous: *model.allow_anonymous(),
            expired_at: *model.expired_at(),
        }
    }

    fn to_mysqldb_model(&self) -> TokenMysqlModel {
        TokenMysqlModel::new(
            &self.id,
            &self.created_at,
            &self.updated_at,
            &self.project_id,
            &self.admin_id,
            &self.token,
            &self.allow_anonymous,
            &self.expired_at,
        )
    }

    fn from_sqlitedb_model(model: &TokenSqliteModel) -> Self {
        Self {
            id: *model.id(),
            created_at: *model.created_at(),
            updated_at: *model.updated_at(),
            project_id: *model.project_id(),
            admin_id: *model.admin_id(),
            token: model.token().to_owned(),
            allow_anonymous: *model.allow_anonymous(),
            expired_at: *model.expired_at(),
        }
    }

    fn to_sqlitedb_model(&self) -> TokenSqliteModel {
        TokenSqliteModel::new(
            &self.id,
            &self.created_at,
            &self.updated_at,
            &self.project_id,
            &self.admin_id,
            &self.token,
            &self.allow_anonymous,
            &self.expired_at,
        )
    }
}
