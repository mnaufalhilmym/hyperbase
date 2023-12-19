use anyhow::Result;
use chrono::{DateTime, Utc};
use hb_db_mysql::{db::MysqlDb, query::project::INSERT as MYSQL_INSERT};
use hb_db_postgresql::{db::PostgresDb, query::project::INSERT as POSTGRES_INSERT};
use hb_db_scylladb::{
    db::ScyllaDb,
    model::project::ProjectModel as ProjectScyllaModel,
    query::project::{
        DELETE as SCYLLA_DELETE, INSERT as SCYLLA_INSERT, SELECT as SCYLLA_SELECT,
        SELECT_MANY_BY_ADMIN_ID as SCYLLA_SELECT_MANY_BY_ADMIN_ID, UPDATE as SCYLLA_UPDATE,
    },
};
use hb_db_sqlite::{db::SqliteDb, query::project::INSERT as SQLITE_INSERT};
use scylla::{frame::value::Timestamp, transport::session::TypedRowIter};
use uuid::Uuid;

use crate::{
    util::conversion::{datetime_to_duration_since_epoch, duration_since_epoch_to_datetime},
    Db,
};

pub struct ProjectDao {
    id: Uuid,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    admin_id: Uuid,
    name: String,
}

impl ProjectDao {
    pub fn new(admin_id: &Uuid, name: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            admin_id: *admin_id,
            name: name.to_owned(),
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

    pub fn admin_id(&self) -> &Uuid {
        &self.admin_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
    }

    pub async fn db_insert(&self, db: &Db) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => Self::scylladb_insert(self, db).await,
            Db::PostgresqlDb(db) => Self::postgresdb_insert(self, db).await,
            Db::MysqlDb(db) => Self::mysqldb_insert(self, db).await,
            Db::SqliteDb(db) => Self::sqlitedb_insert(self, db).await,
        }
    }

    pub async fn db_select(db: &Db, id: &Uuid) -> Result<Self> {
        match db {
            Db::ScyllaDb(db) => Ok(Self::from_scylladb_model(
                &Self::scylladb_select(db, id).await?,
            )?),
            Db::PostgresqlDb(_) => todo!(),
            Db::MysqlDb(_) => todo!(),
            Db::SqliteDb(_) => todo!(),
        }
    }

    pub async fn db_select_many_by_admin_id(db: &Db, admin_id: &Uuid) -> Result<Vec<Self>> {
        match db {
            Db::ScyllaDb(db) => {
                let mut projects_data = Vec::new();
                let projects = Self::scylladb_select_many_by_admin_id(db, admin_id).await?;
                for project in projects {
                    if let Ok(model) = &project {
                        projects_data.push(Self::from_scylladb_model(model)?);
                    } else if let Err(err) = project {
                        return Err(err.into());
                    }
                }
                Ok(projects_data)
            }
            Db::PostgresqlDb(_) => todo!(),
            Db::MysqlDb(_) => todo!(),
            Db::SqliteDb(_) => todo!(),
        }
    }

    pub async fn db_update(&mut self, db: &Db) -> Result<()> {
        self.updated_at = Utc::now();
        match db {
            Db::ScyllaDb(db) => Self::scylladb_update(self, db).await,
            Db::PostgresqlDb(_) => todo!(),
            Db::MysqlDb(_) => todo!(),
            Db::SqliteDb(_) => todo!(),
        }
    }

    pub async fn db_delete(db: &Db, id: &Uuid) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => Self::scylladb_delete(db, id).await,
            Db::PostgresqlDb(_) => todo!(),
            Db::MysqlDb(_) => todo!(),
            Db::SqliteDb(_) => todo!(),
        }
    }

    async fn scylladb_insert(&self, db: &ScyllaDb) -> Result<()> {
        db.execute(SCYLLA_INSERT, &self.to_scylladb_model()).await?;
        Ok(())
    }

    async fn scylladb_select(db: &ScyllaDb, id: &Uuid) -> Result<ProjectScyllaModel> {
        Ok(db
            .execute(SCYLLA_SELECT, [id].as_ref())
            .await?
            .first_row_typed::<ProjectScyllaModel>()?)
    }

    async fn scylladb_select_many_by_admin_id(
        db: &ScyllaDb,
        admin_id: &Uuid,
    ) -> Result<TypedRowIter<ProjectScyllaModel>> {
        Ok(db
            .execute(SCYLLA_SELECT_MANY_BY_ADMIN_ID, [admin_id].as_ref())
            .await?
            .rows_typed::<ProjectScyllaModel>()?)
    }

    async fn scylladb_update(&self, db: &ScyllaDb) -> Result<()> {
        db.execute(SCYLLA_UPDATE, &(&self.updated_at, &self.name, &self.id))
            .await?;
        Ok(())
    }

    async fn scylladb_delete(db: &ScyllaDb, id: &Uuid) -> Result<()> {
        db.execute(SCYLLA_DELETE, [id].as_ref()).await?;
        Ok(())
    }

    async fn postgresdb_insert(&self, db: &PostgresDb) -> Result<()> {
        db.execute(
            sqlx::query(POSTGRES_INSERT)
                .bind(&self.id)
                .bind(&self.created_at)
                .bind(&self.updated_at)
                .bind(&self.admin_id)
                .bind(&self.name),
        )
        .await?;
        Ok(())
    }

    async fn mysqldb_insert(&self, db: &MysqlDb) -> Result<()> {
        db.execute(
            sqlx::query(MYSQL_INSERT)
                .bind(&self.id)
                .bind(&self.created_at)
                .bind(&self.updated_at)
                .bind(&self.admin_id)
                .bind(&self.name),
        )
        .await?;
        Ok(())
    }

    async fn sqlitedb_insert(&self, db: &SqliteDb) -> Result<()> {
        db.execute(
            sqlx::query(SQLITE_INSERT)
                .bind(&self.id)
                .bind(&self.created_at)
                .bind(&self.updated_at)
                .bind(&self.admin_id)
                .bind(&self.name),
        )
        .await?;
        Ok(())
    }

    fn from_scylladb_model(model: &ProjectScyllaModel) -> Result<Self> {
        Ok(Self {
            id: *model.id(),
            created_at: duration_since_epoch_to_datetime(&model.created_at().0)?,
            updated_at: duration_since_epoch_to_datetime(&model.updated_at().0)?,
            admin_id: *model.admin_id(),
            name: model.name().to_owned(),
        })
    }

    fn to_scylladb_model(&self) -> ProjectScyllaModel {
        ProjectScyllaModel::new(
            &self.id,
            &Timestamp(datetime_to_duration_since_epoch(&self.created_at)),
            &Timestamp(datetime_to_duration_since_epoch(&self.updated_at)),
            &self.admin_id,
            &self.name,
        )
    }
}
