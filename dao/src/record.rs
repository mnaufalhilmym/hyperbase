use std::collections::hash_map::Keys;

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use anyhow::{Error, Result};
use hb_db_mysql::{
    db::MysqlDb,
    model::{
        collection::SchemaFieldPropsModel as SchemaFieldPropsMysqlModel,
        system::{
            COMPARISON_OPERATOR as MYSQL_COMPARISON_OPERATOR,
            LOGICAL_OPERATOR as MYSQL_LOGICAL_OPERATOR, ORDER_TYPE as MYSQL_ORDER_TYPE,
        },
    },
    query::{record as mysql_record, system::COUNT_TABLE as MYSQL_COUNT_TABLE},
};
use hb_db_postgresql::{
    db::PostgresDb,
    model::{
        collection::SchemaFieldPropsModel as SchemaFieldPropsPostgresModel,
        system::{
            COMPARISON_OPERATOR as POSTGRES_COMPARISON_OPERATOR,
            LOGICAL_OPERATOR as POSTGRES_LOGICAL_OPERATOR, ORDER_TYPE as POSTGRES_ORDER_TYPE,
        },
    },
    query::{record as postgres_record, system::COUNT_TABLE as POSTGRES_COUNT_TABLE},
};
use hb_db_scylladb::{
    db::ScyllaDb,
    model::{
        collection::SchemaFieldPropsModel as SchemaFieldPropsScyllaModel,
        system::{
            COMPARISON_OPERATOR as SCYLLA_COMPARISON_OPERATOR,
            LOGICAL_OPERATOR as SCYLLA_LOGICAL_OPERATOR, ORDER_TYPE as SCYLLA_ORDER_TYPE,
        },
    },
    query::{record as scylla_record, system::COUNT_TABLE as SCYLLA_COUNT_TABLE},
};
use hb_db_sqlite::{
    db::SqliteDb,
    model::{
        collection::SchemaFieldPropsModel as SchemaFieldPropsSqliteModel,
        system::{
            COMPARISON_OPERATOR as SQLITE_COMPARISON_OPERATOR,
            LOGICAL_OPERATOR as SQLITE_LOGICAL_OPERATOR, ORDER_TYPE as SQLITE_ORDER_TYPE,
        },
    },
    query::{record as sqlite_record, system::COUNT_TABLE as SQLITE_COUNT_TABLE},
};
use scylla::{frame::response::result::CqlValue as ScyllaCqlValue, serialize::value::SerializeCql};
use uuid::Uuid;

use crate::{
    collection::{CollectionDao, SchemaFieldProps},
    value::{ColumnKind, ColumnValue},
    Db,
};

pub struct RecordDao {
    table_name: String,
    data: HashMap<String, ColumnValue>,
}

impl RecordDao {
    pub fn new(collection_id: &Uuid, capacity: &Option<usize>) -> Self {
        let mut data = HashMap::with_capacity(match capacity {
            Some(capacity) => capacity + 1,
            None => 1,
        });
        data.insert("_id".to_owned(), ColumnValue::Uuid(Some(Uuid::now_v7())));

        Self {
            table_name: Self::new_table_name(collection_id),
            data,
        }
    }

    pub fn new_table_name(collection_id: &Uuid) -> String {
        "record_".to_owned() + &collection_id.to_string().replace("-", "")
    }

    pub fn table_name(&self) -> &str {
        &self.table_name
    }

    pub fn data(&self) -> &HashMap<String, ColumnValue> {
        &self.data
    }

    pub fn get(&self, key: &str) -> Option<&ColumnValue> {
        self.data.get(key)
    }

    pub fn keys(&self) -> Keys<'_, String, ColumnValue> {
        self.data.keys()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn upsert(&mut self, key: &str, value: &ColumnValue) {
        self.data.insert(key.to_owned(), value.to_owned());
    }

    pub async fn db_create_table(db: &Db, collection: &CollectionDao) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => {
                Self::scylladb_create_table(
                    db,
                    collection.id(),
                    &collection
                        .schema_fields()
                        .iter()
                        .map(|(field_name, field_props)| {
                            (field_name.clone(), field_props.to_scylladb_model())
                        })
                        .collect::<HashMap<_, _>>(),
                )
                .await
            }
            Db::PostgresqlDb(db) => {
                Self::postgresdb_create_table(
                    db,
                    collection.id(),
                    &collection
                        .schema_fields()
                        .iter()
                        .map(|(field_name, field_props)| {
                            (field_name.clone(), field_props.to_postgresdb_model())
                        })
                        .collect::<HashMap<_, _>>(),
                )
                .await
            }
            Db::MysqlDb(db) => {
                Self::mysqldb_create_table(
                    db,
                    collection.id(),
                    &collection
                        .schema_fields()
                        .iter()
                        .map(|(field_name, field_props)| {
                            (field_name.clone(), field_props.to_mysqldb_model())
                        })
                        .collect::<HashMap<_, _>>(),
                )
                .await
            }
            Db::SqliteDb(db) => {
                Self::sqlitedb_create_table(
                    db,
                    collection.id(),
                    &collection
                        .schema_fields()
                        .iter()
                        .map(|(field_name, field_props)| {
                            (field_name.clone(), field_props.to_sqlitedb_model())
                        })
                        .collect::<HashMap<_, _>>(),
                )
                .await
            }
        }
    }

    pub async fn db_drop_table(db: &Db, collection_id: &Uuid) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => Self::scylladb_drop_table(db, collection_id).await,
            Db::PostgresqlDb(db) => Self::postgresdb_drop_table(db, collection_id).await,
            Db::MysqlDb(db) => Self::mysqldb_drop_table(db, collection_id).await,
            Db::SqliteDb(db) => Self::sqlite_drop_table(db, collection_id).await,
        }
    }

    pub async fn db_check_table_existence(db: &Db, collection_id: &Uuid) -> Result<bool> {
        match db {
            Db::ScyllaDb(db) => Self::scylladb_check_table_existence(db, collection_id).await,
            Db::PostgresqlDb(db) => Self::postgresdb_check_table_existence(db, collection_id).await,
            Db::MysqlDb(db) => Self::mysqldb_check_table_existence(db, collection_id).await,
            Db::SqliteDb(db) => Self::sqlitedb_check_table_existence(db, collection_id).await,
        }
    }

    pub async fn db_check_table_must_exist(db: &Db, collection_id: &Uuid) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => {
                match Self::scylladb_check_table_existence(db, collection_id).await? {
                    true => Ok(()),
                    false => Err(Error::msg(format!(
                        "Collection '{collection_id}' doesn't exist"
                    ))),
                }
            }
            Db::PostgresqlDb(db) => {
                match Self::postgresdb_check_table_existence(db, collection_id).await? {
                    true => Ok(()),
                    false => Err(Error::msg(format!(
                        "Collection '{collection_id}' doesn't exist"
                    ))),
                }
            }
            Db::MysqlDb(db) => {
                match Self::mysqldb_check_table_existence(db, collection_id).await? {
                    true => Ok(()),
                    false => Err(Error::msg(format!(
                        "Collection '{collection_id}' doesn't exist"
                    ))),
                }
            }
            Db::SqliteDb(db) => {
                match Self::sqlitedb_check_table_existence(db, collection_id).await? {
                    true => Ok(()),
                    false => Err(Error::msg(format!(
                        "Collection '{collection_id}' doesn't exist"
                    ))),
                }
            }
        }
    }

    pub async fn db_add_columns(
        db: &Db,
        collection_id: &Uuid,
        columns: &HashMap<String, SchemaFieldProps>,
    ) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => {
                Self::scylladb_add_columns(
                    db,
                    collection_id,
                    &columns
                        .iter()
                        .map(|(col, col_props)| (col.to_owned(), col_props.to_scylladb_model()))
                        .collect(),
                )
                .await
            }
            Db::PostgresqlDb(db) => {
                Self::postgresdb_add_columns(
                    db,
                    collection_id,
                    &columns
                        .iter()
                        .map(|(col, col_props)| (col.to_owned(), col_props.to_postgresdb_model()))
                        .collect(),
                )
                .await
            }
            Db::MysqlDb(db) => {
                Self::mysqldb_add_columns(
                    db,
                    collection_id,
                    &columns
                        .iter()
                        .map(|(col, col_props)| (col.to_owned(), col_props.to_mysqldb_model()))
                        .collect(),
                )
                .await
            }
            Db::SqliteDb(db) => {
                Self::sqlitedb_add_columns(
                    db,
                    collection_id,
                    &columns
                        .iter()
                        .map(|(col, col_props)| (col.to_owned(), col_props.to_sqlitedb_model()))
                        .collect(),
                )
                .await
            }
        }
    }

    pub async fn db_drop_columns(
        db: &Db,
        collection_id: &Uuid,
        column_names: &HashSet<String>,
    ) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => Self::scylladb_drop_columns(db, collection_id, column_names).await,
            Db::PostgresqlDb(db) => {
                Self::postgresdb_drop_columns(db, collection_id, column_names).await
            }
            Db::MysqlDb(db) => Self::mysqldb_drop_columns(db, collection_id, column_names).await,
            Db::SqliteDb(db) => Self::sqlitedb_drop_columns(db, collection_id, column_names).await,
        }
    }

    pub async fn db_change_columns_type(
        db: &Db,
        collection_id: &Uuid,
        columns: &HashMap<String, SchemaFieldProps>,
    ) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => {
                Self::scylladb_change_columns_type(
                    db,
                    collection_id,
                    &columns
                        .iter()
                        .map(|(col, col_props)| (col.to_owned(), col_props.to_scylladb_model()))
                        .collect(),
                )
                .await
            }
            Db::PostgresqlDb(db) => {
                Self::postgresdb_change_columns_type(
                    db,
                    collection_id,
                    &columns
                        .iter()
                        .map(|(col, col_props)| (col.to_owned(), col_props.to_postgresdb_model()))
                        .collect(),
                )
                .await
            }
            Db::MysqlDb(db) => {
                Self::mysqldb_change_columns_type(
                    db,
                    collection_id,
                    &columns
                        .iter()
                        .map(|(col, col_props)| (col.to_owned(), col_props.to_mysqldb_model()))
                        .collect(),
                )
                .await
            }
            Db::SqliteDb(db) => {
                Self::sqlitedb_change_columns_type(
                    db,
                    collection_id,
                    &columns
                        .iter()
                        .map(|(col, col_props)| (col.to_owned(), col_props.to_sqlitedb_model()))
                        .collect(),
                )
                .await
            }
        }
    }

    pub async fn db_create_index(db: &Db, collection_id: &Uuid, index: &str) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => Self::scylladb_create_index(db, collection_id, index).await,
            Db::PostgresqlDb(db) => Self::postgresdb_create_index(db, collection_id, index).await,
            Db::MysqlDb(db) => Self::mysqldb_create_index(db, collection_id, index).await,
            Db::SqliteDb(db) => Self::sqlitedb_create_index(db, collection_id, index).await,
        }
    }

    pub async fn db_drop_index(db: &Db, collection_id: &Uuid, index: &str) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => Self::scylladb_drop_index(db, collection_id, index).await,
            Db::PostgresqlDb(db) => Self::postgresdb_drop_index(db, collection_id, index).await,
            Db::MysqlDb(db) => Self::mysqldb_drop_index(db, collection_id, index).await,
            Db::SqliteDb(db) => Self::sqlitedb_drop_index(db, collection_id, index).await,
        }
    }

    pub async fn db_insert(&self, db: &Db) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => Self::scylladb_insert(self, db).await,
            Db::PostgresqlDb(db) => Self::postgresdb_insert(self, db).await,
            Db::MysqlDb(db) => Self::mysqldb_insert(self, db).await,
            Db::SqliteDb(db) => Self::sqlitedb_insert(self, db).await,
        }
    }

    pub async fn db_select(db: &Db, collection_data: &CollectionDao, id: &Uuid) -> Result<Self> {
        match db {
            Db::ScyllaDb(db) => {
                let table_name = Self::new_table_name(collection_data.id());

                let mut columns = Vec::with_capacity(collection_data.schema_fields().len() + 1);
                let mut columns_props =
                    Vec::with_capacity(collection_data.schema_fields().len() + 1);

                columns.push("_id");
                columns_props.push(SchemaFieldProps::new(&ColumnKind::Uuid, &true));

                for (column, props) in collection_data.schema_fields() {
                    columns.push(column);
                    columns_props.push(*props)
                }

                let scylladb_data = Self::scylladb_select(db, &table_name, &columns, id).await?;

                let mut data = HashMap::with_capacity(scylladb_data.len());
                for (idx, value) in scylladb_data.iter().enumerate() {
                    match value {
                        Some(value) => {
                            match ColumnValue::from_scylladb_model(columns_props[idx].kind(), value)
                            {
                                Ok(value) => data.insert(columns[idx].to_owned(), value),
                                Err(err) => return Err(err.into()),
                            }
                        }
                        None => data.insert(
                            columns[idx].to_owned(),
                            ColumnValue::none(columns_props[idx].kind()),
                        ),
                    };
                }

                Ok(Self { table_name, data })
            }
            Db::PostgresqlDb(db) => {
                let table_name = Self::new_table_name(collection_data.id());

                let mut columns = Vec::with_capacity(collection_data.schema_fields().len() + 1);
                columns.push("_id");
                for column in collection_data.schema_fields().keys() {
                    columns.push(column);
                }

                let postgresdb_data =
                    Self::postgresdb_select(db, &table_name, &columns, id).await?;

                let mut data = HashMap::with_capacity(columns.len());
                data.insert(
                    "_id".to_owned(),
                    ColumnValue::from_postgresdb_model(&ColumnKind::Uuid, "_id", &postgresdb_data)?,
                );
                for (field, field_props) in collection_data.schema_fields() {
                    data.insert(
                        field.to_owned(),
                        ColumnValue::from_postgresdb_model(
                            field_props.kind(),
                            field,
                            &postgresdb_data,
                        )?,
                    );
                }

                Ok(Self { table_name, data })
            }
            Db::MysqlDb(db) => {
                let table_name = Self::new_table_name(collection_data.id());

                let mut columns = Vec::with_capacity(collection_data.schema_fields().len() + 1);
                columns.push("_id");
                for column in collection_data.schema_fields().keys() {
                    columns.push(column);
                }

                let mysqldb_data = Self::mysqldb_select(db, &table_name, &columns, id).await?;

                let mut data = HashMap::with_capacity(columns.len());
                data.insert(
                    "_id".to_owned(),
                    ColumnValue::from_mysqldb_model(&ColumnKind::Uuid, "_id", &mysqldb_data)?,
                );
                for (field, field_props) in collection_data.schema_fields() {
                    data.insert(
                        field.to_owned(),
                        ColumnValue::from_mysqldb_model(field_props.kind(), field, &mysqldb_data)?,
                    );
                }

                Ok(Self { table_name, data })
            }
            Db::SqliteDb(db) => {
                let table_name = Self::new_table_name(collection_data.id());

                let mut columns = Vec::with_capacity(collection_data.schema_fields().len() + 1);
                columns.push("_id");
                for column in collection_data.schema_fields().keys() {
                    columns.push(column);
                }

                let sqlitedb_data = Self::sqlitedb_select(db, &table_name, &columns, id).await?;

                let mut data = HashMap::with_capacity(columns.len());
                data.insert(
                    "_id".to_owned(),
                    ColumnValue::from_sqlitedb_model(&ColumnKind::Uuid, "_id", &sqlitedb_data)?,
                );
                for (field, field_props) in collection_data.schema_fields() {
                    data.insert(
                        field.to_owned(),
                        ColumnValue::from_sqlitedb_model(
                            field_props.kind(),
                            field,
                            &sqlitedb_data,
                        )?,
                    );
                }

                Ok(Self { table_name, data })
            }
        }
    }

    pub async fn db_select_many(
        db: &Db,
        collection_data: &CollectionDao,
        filters: &RecordFilters,
        groups: &Vec<&str>,
        orders: &Vec<RecordOrder>,
        pagination: &RecordPagination,
    ) -> Result<(Vec<Self>, i64)> {
        match db {
            Db::ScyllaDb(db) => {
                let table_name = Self::new_table_name(collection_data.id());

                let mut columns = Vec::with_capacity(collection_data.schema_fields().len() + 1);
                let mut columns_props =
                    Vec::with_capacity(collection_data.schema_fields().len() + 1);

                columns.push("_id");
                columns_props.push(SchemaFieldProps::new(&ColumnKind::Uuid, &true));

                for (column, props) in collection_data.schema_fields() {
                    columns.push(column);
                    columns_props.push(*props)
                }

                let (scylladb_data_many, total) = Self::scylladb_select_many(
                    db,
                    &table_name,
                    &columns,
                    filters,
                    groups,
                    orders,
                    pagination,
                )
                .await?;

                let mut data_many = Vec::with_capacity(scylladb_data_many.len());
                for scylladb_data in scylladb_data_many {
                    let mut data = HashMap::with_capacity(scylladb_data.len());
                    for (idx, value) in scylladb_data.iter().enumerate() {
                        match value {
                            Some(value) => match ColumnValue::from_scylladb_model(
                                columns_props[idx].kind(),
                                value,
                            ) {
                                Ok(value) => data.insert(columns[idx].to_owned(), value),
                                Err(err) => return Err(err.into()),
                            },
                            None => data.insert(
                                columns[idx].to_owned(),
                                ColumnValue::none(columns_props[idx].kind()),
                            ),
                        };
                    }
                    data_many.push(Self {
                        table_name: table_name.to_owned(),
                        data,
                    });
                }

                Ok((data_many, total))
            }
            Db::PostgresqlDb(db) => {
                let table_name = Self::new_table_name(collection_data.id());

                let mut columns = Vec::with_capacity(collection_data.schema_fields().len() + 1);
                columns.push("_id");
                for column in collection_data.schema_fields().keys() {
                    columns.push(column);
                }

                let (postgres_data_many, total) = Self::postgresdb_select_many(
                    db,
                    &table_name,
                    &columns,
                    filters,
                    groups,
                    orders,
                    pagination,
                )
                .await?;

                let mut data_many = Vec::with_capacity(postgres_data_many.len());
                for postgres_data in &postgres_data_many {
                    let mut data: std::collections::HashMap<_, _, ahash::RandomState> =
                        HashMap::with_capacity(columns.len());
                    data.insert(
                        "_id".to_owned(),
                        ColumnValue::from_postgresdb_model(
                            &ColumnKind::Uuid,
                            "_id",
                            postgres_data,
                        )?,
                    );
                    for (field, field_props) in collection_data.schema_fields() {
                        data.insert(
                            field.to_owned(),
                            ColumnValue::from_postgresdb_model(
                                field_props.kind(),
                                field,
                                postgres_data,
                            )?,
                        );
                    }
                    data_many.push(Self {
                        table_name: table_name.to_owned(),
                        data,
                    })
                }

                Ok((data_many, total))
            }
            Db::MysqlDb(db) => {
                let table_name = Self::new_table_name(collection_data.id());

                let mut columns = Vec::with_capacity(collection_data.schema_fields().len() + 1);
                columns.push("_id");
                for column in collection_data.schema_fields().keys() {
                    columns.push(column);
                }

                let (mysql_data_many, total) = Self::mysqldb_select_many(
                    db,
                    &table_name,
                    &columns,
                    filters,
                    groups,
                    orders,
                    pagination,
                )
                .await?;

                let mut data_many = Vec::with_capacity(mysql_data_many.len());
                for mysql_data in &mysql_data_many {
                    let mut data: std::collections::HashMap<_, _, ahash::RandomState> =
                        HashMap::with_capacity(columns.len());
                    data.insert(
                        "_id".to_owned(),
                        ColumnValue::from_mysqldb_model(&ColumnKind::Uuid, "_id", mysql_data)?,
                    );
                    for (field, field_props) in collection_data.schema_fields() {
                        data.insert(
                            field.to_owned(),
                            ColumnValue::from_mysqldb_model(field_props.kind(), field, mysql_data)?,
                        );
                    }
                    data_many.push(Self {
                        table_name: table_name.to_owned(),
                        data,
                    })
                }

                Ok((data_many, total))
            }
            Db::SqliteDb(db) => {
                let table_name = Self::new_table_name(collection_data.id());

                let mut columns = Vec::with_capacity(collection_data.schema_fields().len() + 1);
                columns.push("_id");
                for column in collection_data.schema_fields().keys() {
                    columns.push(column);
                }

                let (sqlite_data_many, total) = Self::sqlitedb_select_many(
                    db,
                    &table_name,
                    &columns,
                    filters,
                    groups,
                    orders,
                    pagination,
                )
                .await?;

                let mut data_many = Vec::with_capacity(sqlite_data_many.len());
                for sqlite_data in &sqlite_data_many {
                    let mut data: std::collections::HashMap<_, _, ahash::RandomState> =
                        HashMap::with_capacity(columns.len());
                    data.insert(
                        "_id".to_owned(),
                        ColumnValue::from_sqlitedb_model(&ColumnKind::Uuid, "_id", sqlite_data)?,
                    );
                    for (field, field_props) in collection_data.schema_fields() {
                        data.insert(
                            field.to_owned(),
                            ColumnValue::from_sqlitedb_model(
                                field_props.kind(),
                                field,
                                sqlite_data,
                            )?,
                        );
                    }
                    data_many.push(Self {
                        table_name: table_name.to_owned(),
                        data,
                    })
                }

                Ok((data_many, total))
            }
        }
    }

    pub async fn db_update(&self, db: &Db) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => Self::scylladb_update(self, db).await,
            Db::PostgresqlDb(db) => Self::postgresdb_update(self, db).await,
            Db::MysqlDb(db) => Self::mysqldb_update(self, db).await,
            Db::SqliteDb(db) => Self::sqlitedb_update(self, db).await,
        }
    }

    pub async fn db_delete(db: &Db, collection_id: &Uuid, id: &Uuid) -> Result<()> {
        match db {
            Db::ScyllaDb(db) => Self::scylladb_delete(db, collection_id, id).await,
            Db::PostgresqlDb(db) => Self::postgresdb_delete(db, collection_id, id).await,
            Db::MysqlDb(db) => Self::mysqldb_delete(db, collection_id, id).await,
            Db::SqliteDb(db) => Self::sqlitedb_delete(db, collection_id, id).await,
        }
    }

    async fn scylladb_create_table(
        db: &ScyllaDb,
        collection_id: &Uuid,
        schema_fields: &HashMap<String, SchemaFieldPropsScyllaModel>,
    ) -> Result<()> {
        db.session_query(
            &scylla_record::create_table(&Self::new_table_name(collection_id), schema_fields),
            &[],
        )
        .await?;
        Ok(())
    }

    async fn scylladb_drop_table(db: &ScyllaDb, collection_id: &Uuid) -> Result<()> {
        db.session_query(
            &scylla_record::drop_table(&RecordDao::new_table_name(collection_id)),
            &[],
        )
        .await?;
        Ok(())
    }

    async fn scylladb_check_table_existence(db: &ScyllaDb, collection_id: &Uuid) -> Result<bool> {
        Ok(db
            .session_query(
                SCYLLA_COUNT_TABLE,
                [&RecordDao::new_table_name(collection_id)].as_ref(),
            )
            .await?
            .first_row_typed::<(i64,)>()?
            .0
            > 0)
    }

    async fn scylladb_add_columns(
        db: &ScyllaDb,
        collection_id: &Uuid,
        columns: &HashMap<String, SchemaFieldPropsScyllaModel>,
    ) -> Result<()> {
        db.session_query(
            &scylla_record::add_columns(&Self::new_table_name(collection_id), columns),
            &[],
        )
        .await?;
        Ok(())
    }

    async fn scylladb_drop_columns(
        db: &ScyllaDb,
        collection_id: &Uuid,
        column_names: &HashSet<String>,
    ) -> Result<()> {
        db.session_query(
            &scylla_record::drop_columns(&Self::new_table_name(collection_id), column_names),
            &[],
        )
        .await?;
        Ok(())
    }

    async fn scylladb_change_columns_type(
        db: &ScyllaDb,
        collection_id: &Uuid,
        columns: &HashMap<String, SchemaFieldPropsScyllaModel>,
    ) -> Result<()> {
        db.session_query(
            &scylla_record::change_columns_type(&Self::new_table_name(collection_id), columns),
            &[],
        )
        .await?;
        Ok(())
    }

    async fn scylladb_create_index(db: &ScyllaDb, collection_id: &Uuid, index: &str) -> Result<()> {
        db.session_query(
            &scylla_record::create_index(&Self::new_table_name(collection_id), index),
            &[],
        )
        .await?;
        Ok(())
    }

    async fn scylladb_drop_index(db: &ScyllaDb, collection_id: &Uuid, index: &str) -> Result<()> {
        db.session_query(
            &scylla_record::drop_index(&Self::new_table_name(collection_id), index),
            &[],
        )
        .await?;
        Ok(())
    }

    async fn scylladb_insert(&self, db: &ScyllaDb) -> Result<()> {
        let mut columns: Vec<_> = Vec::with_capacity(self.data.len());
        let mut values = Vec::with_capacity(self.data.len());
        for (col, val) in &self.data {
            columns.push(col.as_str());
            values.push(val.to_scylladb_model()?);
        }
        db.execute(&scylla_record::insert(&self.table_name, &columns), &values)
            .await?;
        Ok(())
    }

    async fn scylladb_select(
        db: &ScyllaDb,
        table_name: &str,
        columns: &Vec<&str>,
        id: &Uuid,
    ) -> Result<Vec<Option<ScyllaCqlValue>>> {
        Ok(db
            .execute(&scylla_record::select(table_name, columns), [id].as_ref())
            .await?
            .first_row()?
            .columns)
    }

    async fn scylladb_select_many(
        db: &ScyllaDb,
        table_name: &str,
        columns: &Vec<&str>,
        filters: &RecordFilters,
        groups: &Vec<&str>,
        orders: &Vec<RecordOrder>,
        pagination: &RecordPagination,
    ) -> Result<(Vec<Vec<Option<ScyllaCqlValue>>>, i64)> {
        let filter = filters.scylladb_filter_query(&None, 0)?;

        let mut order = Vec::with_capacity(orders.len());
        for o in orders {
            if SCYLLA_ORDER_TYPE.contains(&o.kind.to_uppercase().as_str()) {
                order.push((o.field.as_str(), o.kind.as_str()));
            } else {
                return Err(Error::msg(format!(
                    "Order type '{}' is not supported",
                    &o.kind
                )));
            }
        }

        let mut values = filters.scylladb_values()?;
        if let Some(limit) = pagination.limit() {
            values.push(Box::new(limit))
        }
        let total_values = filters.scylladb_values()?;

        let query_select_many = scylla_record::select_many(
            table_name,
            columns,
            &filter,
            groups,
            &order,
            &pagination.limit().is_some(),
        );
        let query_total = scylla_record::count(table_name, &filter);

        let (data, total) = tokio::try_join!(
            db.execute(&query_select_many, &values),
            db.execute(&query_total, &total_values)
        )?;

        Ok((
            data.rows()?
                .iter()
                .map(|row| row.columns.to_owned())
                .collect(),
            total.first_row_typed::<(i64,)>()?.0,
        ))
    }

    async fn scylladb_update(&self, db: &ScyllaDb) -> Result<()> {
        let mut columns = Vec::with_capacity(self.data.len());
        let mut values = Vec::with_capacity(self.data.len());
        for (col, val) in &self.data {
            if col != "_id" {
                columns.push(col.as_str());
                values.push(val.to_scylladb_model()?);
            }
        }
        match self.data.get("_id") {
            Some(id) => values.push(id.to_scylladb_model()?),
            None => return Err(Error::msg("Id is undefined")),
        }
        db.execute(&scylla_record::update(&self.table_name, &columns), &values)
            .await?;
        Ok(())
    }

    async fn scylladb_delete(db: &ScyllaDb, collection_id: &Uuid, id: &Uuid) -> Result<()> {
        let mut column = HashSet::<String>::with_capacity(1);
        column.insert("_id".to_owned());
        db.execute(
            &scylla_record::delete(&Self::new_table_name(collection_id), &column),
            [id].as_ref(),
        )
        .await?;
        Ok(())
    }

    async fn postgresdb_create_table(
        db: &PostgresDb,
        collection_id: &Uuid,
        schema_fields: &HashMap<String, SchemaFieldPropsPostgresModel>,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&postgres_record::create_table(
            &Self::new_table_name(collection_id),
            schema_fields,
        )))
        .await?;
        Ok(())
    }

    async fn postgresdb_drop_table(db: &PostgresDb, collection_id: &Uuid) -> Result<()> {
        db.execute_unprepared(sqlx::query(&postgres_record::drop_table(
            &Self::new_table_name(collection_id),
        )))
        .await?;
        Ok(())
    }

    async fn postgresdb_check_table_existence(
        db: &PostgresDb,
        collection_id: &Uuid,
    ) -> Result<bool> {
        Ok(db
            .fetch_one_unprepared::<(i64,)>(
                sqlx::query_as(POSTGRES_COUNT_TABLE)
                    .bind(&RecordDao::new_table_name(collection_id)),
            )
            .await?
            .0
            > 0)
    }

    async fn postgresdb_add_columns(
        db: &PostgresDb,
        collection_id: &Uuid,
        columns: &HashMap<String, SchemaFieldPropsPostgresModel>,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&postgres_record::add_columns(
            &Self::new_table_name(collection_id),
            columns,
        )))
        .await?;
        Ok(())
    }

    async fn postgresdb_drop_columns(
        db: &PostgresDb,
        collection_id: &Uuid,
        column_names: &HashSet<String>,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&postgres_record::drop_columns(
            &Self::new_table_name(collection_id),
            column_names,
        )))
        .await?;
        Ok(())
    }

    async fn postgresdb_change_columns_type(
        db: &PostgresDb,
        collection_id: &Uuid,
        columns: &HashMap<String, SchemaFieldPropsPostgresModel>,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&postgres_record::change_columns_type(
            &Self::new_table_name(collection_id),
            columns,
        )))
        .await?;
        Ok(())
    }

    async fn postgresdb_create_index(
        db: &PostgresDb,
        collection_id: &Uuid,
        index: &str,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&postgres_record::create_index(
            &Self::new_table_name(collection_id),
            index,
        )))
        .await?;
        Ok(())
    }

    async fn postgresdb_drop_index(
        db: &PostgresDb,
        collection_id: &Uuid,
        index: &str,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&postgres_record::drop_index(
            &Self::new_table_name(collection_id),
            index,
        )))
        .await?;
        Ok(())
    }

    async fn postgresdb_insert(&self, db: &PostgresDb) -> Result<()> {
        let mut columns = Vec::with_capacity(self.data.len());
        let mut values = Vec::with_capacity(self.data.len());
        for (col, val) in &self.data {
            columns.push(col.as_str());
            values.push(val);
        }
        let query = postgres_record::insert(&self.table_name, &columns);
        let mut query = sqlx::query(&query);
        for val in values {
            query = val.to_postgresdb_model(query)?;
        }
        db.execute(query).await?;
        Ok(())
    }

    async fn postgresdb_select(
        db: &PostgresDb,
        table_name: &str,
        columns: &Vec<&str>,
        id: &Uuid,
    ) -> Result<sqlx::postgres::PgRow> {
        Ok(db
            .fetch_one_row(sqlx::query(&postgres_record::select(table_name, columns)).bind(id))
            .await?)
    }

    async fn postgresdb_select_many(
        db: &PostgresDb,
        table_name: &str,
        columns: &Vec<&str>,
        filters: &RecordFilters,
        groups: &Vec<&str>,
        orders: &Vec<RecordOrder>,
        pagination: &RecordPagination,
    ) -> Result<(Vec<sqlx::postgres::PgRow>, i64)> {
        let mut argument_idx = 1;
        let filter = filters.postgresdb_filter_query(&None, 0, &mut argument_idx)?;

        let mut order = Vec::with_capacity(orders.len());
        for o in orders {
            if POSTGRES_ORDER_TYPE.contains(&o.kind.to_uppercase().as_str()) {
                order.push((o.field.as_str(), o.kind.as_str()));
            } else {
                return Err(Error::msg(format!(
                    "Order type '{}' is not supported",
                    &o.kind
                )));
            }
        }

        let query_select_many = postgres_record::select_many(
            table_name,
            &columns,
            &filter,
            groups,
            &order,
            &pagination.limit().is_some(),
            &argument_idx,
        );
        let mut query_select_many = sqlx::query(&query_select_many);
        let query_total = postgres_record::count(table_name, &filter);
        let mut query_total = sqlx::query_as(&query_total);

        query_select_many = filters.postgresdb_values(query_select_many)?;
        if let Some(limit) = pagination.limit() {
            query_select_many = query_select_many.bind(limit);
        }
        query_total = filters.postgresdb_values_as(query_total)?;

        let (rows, total) = tokio::try_join!(
            db.fetch_all_rows(query_select_many),
            db.fetch_one::<(i64,)>(query_total)
        )?;

        Ok((rows, total.0))
    }

    async fn postgresdb_update(&self, db: &PostgresDb) -> Result<()> {
        let mut columns = Vec::with_capacity(self.data.len());
        let mut values = Vec::with_capacity(self.data.len());
        for (col, val) in &self.data {
            if col != "_id" {
                columns.push(col.as_str());
                values.push(val);
            }
        }
        match self.data.get("_id") {
            Some(id) => values.push(id),
            None => return Err(Error::msg("Id is undefined")),
        }
        let query = postgres_record::update(&self.table_name, &columns);
        let mut query = sqlx::query(&query);
        for val in values {
            query = val.to_postgresdb_model(query)?;
        }
        db.execute(query).await?;
        Ok(())
    }

    async fn postgresdb_delete(db: &PostgresDb, collection_id: &Uuid, id: &Uuid) -> Result<()> {
        let mut column = HashSet::<String>::with_capacity(1);
        column.insert("_id".to_owned());
        db.execute(
            sqlx::query(&postgres_record::delete(
                &Self::new_table_name(collection_id),
                &column,
            ))
            .bind(id),
        )
        .await?;
        Ok(())
    }

    async fn mysqldb_create_table(
        db: &MysqlDb,
        collection_id: &Uuid,
        schema_fields: &HashMap<String, SchemaFieldPropsMysqlModel>,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&mysql_record::create_table(
            &Self::new_table_name(collection_id),
            schema_fields,
        )))
        .await?;
        Ok(())
    }

    async fn mysqldb_drop_table(db: &MysqlDb, collection_id: &Uuid) -> Result<()> {
        db.execute_unprepared(sqlx::query(&mysql_record::drop_table(
            &Self::new_table_name(collection_id),
        )))
        .await?;
        Ok(())
    }

    async fn mysqldb_check_table_existence(db: &MysqlDb, collection_id: &Uuid) -> Result<bool> {
        Ok(db
            .fetch_one_unprepared::<(i64,)>(
                sqlx::query_as(MYSQL_COUNT_TABLE).bind(&RecordDao::new_table_name(collection_id)),
            )
            .await?
            .0
            > 0)
    }

    async fn mysqldb_add_columns(
        db: &MysqlDb,
        collection_id: &Uuid,
        columns: &HashMap<String, SchemaFieldPropsMysqlModel>,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&mysql_record::add_columns(
            &Self::new_table_name(collection_id),
            columns,
        )))
        .await?;
        Ok(())
    }

    async fn mysqldb_drop_columns(
        db: &MysqlDb,
        collection_id: &Uuid,
        column_names: &HashSet<String>,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&mysql_record::drop_columns(
            &Self::new_table_name(collection_id),
            column_names,
        )))
        .await?;
        Ok(())
    }

    async fn mysqldb_change_columns_type(
        db: &MysqlDb,
        collection_id: &Uuid,
        columns: &HashMap<String, SchemaFieldPropsMysqlModel>,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&mysql_record::change_columns_type(
            &Self::new_table_name(collection_id),
            columns,
        )))
        .await?;
        Ok(())
    }

    async fn mysqldb_create_index(db: &MysqlDb, collection_id: &Uuid, index: &str) -> Result<()> {
        let record_table = Self::new_table_name(collection_id);

        let does_index_exist =
            db.fetch_one::<(i64,)>(sqlx::query_as(&mysql_record::count_index(
                &record_table,
                index,
            )))
            .await?
            .0 > 0;

        if !does_index_exist {
            db.execute_unprepared(sqlx::query(&mysql_record::create_index(
                &record_table,
                index,
            )))
            .await?;
        }

        Ok(())
    }

    async fn mysqldb_drop_index(db: &MysqlDb, collection_id: &Uuid, index: &str) -> Result<()> {
        let record_table = Self::new_table_name(collection_id);

        let does_index_exist =
            db.fetch_one::<(i64,)>(sqlx::query_as(&mysql_record::count_index(
                &record_table,
                index,
            )))
            .await?
            .0 > 0;

        if does_index_exist {
            db.execute_unprepared(sqlx::query(&mysql_record::drop_index(
                &Self::new_table_name(collection_id),
                index,
            )))
            .await?;
        }

        Ok(())
    }

    async fn mysqldb_insert(&self, db: &MysqlDb) -> Result<()> {
        let mut columns = Vec::with_capacity(self.data.len());
        let mut values = Vec::with_capacity(self.data.len());
        for (col, val) in &self.data {
            columns.push(col.as_str());
            values.push(val);
        }
        let query = mysql_record::insert(&self.table_name, &columns);
        let mut query = sqlx::query(&query);
        for val in values {
            query = val.to_mysqldb_model(query)?;
        }
        db.execute(query).await?;
        Ok(())
    }

    async fn mysqldb_select(
        db: &MysqlDb,
        table_name: &str,
        columns: &Vec<&str>,
        id: &Uuid,
    ) -> Result<sqlx::mysql::MySqlRow> {
        Ok(db
            .fetch_one_row(sqlx::query(&mysql_record::select(table_name, columns)).bind(id))
            .await?)
    }

    async fn mysqldb_select_many(
        db: &MysqlDb,
        table_name: &str,
        columns: &Vec<&str>,
        filters: &RecordFilters,
        groups: &Vec<&str>,
        orders: &Vec<RecordOrder>,
        pagination: &RecordPagination,
    ) -> Result<(Vec<sqlx::mysql::MySqlRow>, i64)> {
        let filter = filters.mysqldb_filter_query(&None, 0)?;

        let mut order = Vec::with_capacity(orders.len());
        for o in orders {
            if MYSQL_ORDER_TYPE.contains(&o.kind.to_uppercase().as_str()) {
                order.push((o.field.as_str(), o.kind.as_str()));
            } else {
                return Err(Error::msg(format!(
                    "Order type '{}' is not supported",
                    &o.kind
                )));
            }
        }

        let query_select_many = mysql_record::select_many(
            table_name,
            &columns,
            &filter,
            groups,
            &order,
            &pagination.limit().is_some(),
        );
        let mut query_select_many = sqlx::query(&query_select_many);
        let query_total = mysql_record::count(table_name, &filter);
        let mut query_total = sqlx::query_as(&query_total);

        query_select_many = filters.mysqldb_values(query_select_many)?;
        if let Some(limit) = pagination.limit() {
            query_select_many = query_select_many.bind(limit);
        }
        query_total = filters.mysqldb_values_as(query_total)?;

        let (rows, total) = tokio::try_join!(
            db.fetch_all_rows(query_select_many),
            db.fetch_one::<(i64,)>(query_total)
        )?;

        Ok((rows, total.0))
    }

    async fn mysqldb_update(&self, db: &MysqlDb) -> Result<()> {
        let mut columns = Vec::with_capacity(self.data.len());
        let mut values = Vec::with_capacity(self.data.len());
        for (col, val) in &self.data {
            if col != "_id" {
                columns.push(col.as_str());
                values.push(val);
            }
        }
        match self.data.get("_id") {
            Some(id) => values.push(id),
            None => return Err(Error::msg("Id is undefined")),
        }
        let query = mysql_record::update(&self.table_name, &columns);
        let mut query = sqlx::query(&query);
        for val in values {
            query = val.to_mysqldb_model(query)?;
        }
        db.execute(query).await?;
        Ok(())
    }

    async fn mysqldb_delete(db: &MysqlDb, collection_id: &Uuid, id: &Uuid) -> Result<()> {
        let mut column = HashSet::<String>::with_capacity(1);
        column.insert("_id".to_owned());
        db.execute(
            sqlx::query(&mysql_record::delete(
                &Self::new_table_name(collection_id),
                &column,
            ))
            .bind(id),
        )
        .await?;
        Ok(())
    }

    async fn sqlitedb_create_table(
        db: &SqliteDb,
        collection_id: &Uuid,
        schema_fields: &HashMap<String, SchemaFieldPropsSqliteModel>,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&sqlite_record::create_table(
            &Self::new_table_name(collection_id),
            schema_fields,
        )))
        .await?;
        Ok(())
    }

    async fn sqlite_drop_table(db: &SqliteDb, collection_id: &Uuid) -> Result<()> {
        db.execute_unprepared(sqlx::query(&sqlite_record::drop_table(
            &Self::new_table_name(collection_id),
        )))
        .await?;
        Ok(())
    }

    async fn sqlitedb_check_table_existence(db: &SqliteDb, collection_id: &Uuid) -> Result<bool> {
        Ok(db
            .fetch_one_unprepared::<(i64,)>(
                sqlx::query_as(SQLITE_COUNT_TABLE).bind(&RecordDao::new_table_name(collection_id)),
            )
            .await?
            .0
            > 0)
    }

    async fn sqlitedb_add_columns(
        db: &SqliteDb,
        collection_id: &Uuid,
        columns: &HashMap<String, SchemaFieldPropsSqliteModel>,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&sqlite_record::add_columns(
            &Self::new_table_name(collection_id),
            columns,
        )))
        .await?;
        Ok(())
    }

    async fn sqlitedb_drop_columns(
        db: &SqliteDb,
        collection_id: &Uuid,
        column_names: &HashSet<String>,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&sqlite_record::drop_columns(
            &Self::new_table_name(collection_id),
            column_names,
        )))
        .await?;
        Ok(())
    }

    async fn sqlitedb_change_columns_type(
        db: &SqliteDb,
        collection_id: &Uuid,
        columns: &HashMap<String, SchemaFieldPropsSqliteModel>,
    ) -> Result<()> {
        db.execute_unprepared(sqlx::query(&sqlite_record::change_columns_type(
            &Self::new_table_name(collection_id),
            columns,
        )))
        .await?;
        Ok(())
    }

    async fn sqlitedb_create_index(db: &SqliteDb, collection_id: &Uuid, index: &str) -> Result<()> {
        db.execute_unprepared(sqlx::query(&sqlite_record::create_index(
            &Self::new_table_name(collection_id),
            index,
        )))
        .await?;
        Ok(())
    }

    async fn sqlitedb_drop_index(db: &SqliteDb, collection_id: &Uuid, index: &str) -> Result<()> {
        db.execute_unprepared(sqlx::query(&sqlite_record::drop_index(
            &Self::new_table_name(collection_id),
            index,
        )))
        .await?;
        Ok(())
    }

    async fn sqlitedb_insert(&self, db: &SqliteDb) -> Result<()> {
        let mut columns = Vec::with_capacity(self.data.len());
        let mut values = Vec::with_capacity(self.data.len());
        for (col, val) in &self.data {
            columns.push(col.as_str());
            values.push(val);
        }
        let query = sqlite_record::insert(&self.table_name, &columns);
        let mut query = sqlx::query(&query);
        for val in values {
            query = val.to_sqlitedb_model(query)?;
        }
        db.execute(query).await?;
        Ok(())
    }

    async fn sqlitedb_select(
        db: &SqliteDb,
        table_name: &str,
        columns: &Vec<&str>,
        id: &Uuid,
    ) -> Result<sqlx::sqlite::SqliteRow> {
        Ok(db
            .fetch_one_row(sqlx::query(&sqlite_record::select(table_name, columns)).bind(id))
            .await?)
    }

    async fn sqlitedb_select_many(
        db: &SqliteDb,
        table_name: &str,
        columns: &Vec<&str>,
        filters: &RecordFilters,
        groups: &Vec<&str>,
        orders: &Vec<RecordOrder>,
        pagination: &RecordPagination,
    ) -> Result<(Vec<sqlx::sqlite::SqliteRow>, i64)> {
        let filter = filters.sqlitedb_filter_query(&None, 0)?;

        let mut order = Vec::with_capacity(orders.len());
        for o in orders {
            if SQLITE_ORDER_TYPE.contains(&o.kind.to_uppercase().as_str()) {
                order.push((o.field.as_str(), o.kind.as_str()));
            } else {
                return Err(Error::msg(format!(
                    "Order type '{}' is not supported",
                    &o.kind
                )));
            }
        }

        let query_select_many = sqlite_record::select_many(
            table_name,
            &columns,
            &filter,
            groups,
            &order,
            &pagination.limit().is_some(),
        );
        let mut query_select_many = sqlx::query(&query_select_many);
        let query_total = sqlite_record::count(table_name, &filter);
        let mut query_total = sqlx::query_as(&query_total);

        query_select_many = filters.sqlitedb_values(query_select_many)?;
        if let Some(limit) = pagination.limit() {
            query_select_many = query_select_many.bind(limit);
        }
        query_total = filters.sqlitedb_values_as(query_total)?;

        let (rows, total) = tokio::try_join!(
            db.fetch_all_rows(query_select_many),
            db.fetch_one::<(i64,)>(query_total)
        )?;

        Ok((rows, total.0))
    }

    async fn sqlitedb_update(&self, db: &SqliteDb) -> Result<()> {
        let mut columns = Vec::with_capacity(self.data.len());
        let mut values = Vec::with_capacity(self.data.len());
        for (col, val) in &self.data {
            if col != "_id" {
                columns.push(col.as_str());
                values.push(val);
            }
        }
        match self.data.get("_id") {
            Some(id) => values.push(id),
            None => return Err(Error::msg("Id is undefined")),
        }
        let query = sqlite_record::update(&self.table_name, &columns);
        let mut query = sqlx::query(&query);
        for val in values {
            query = val.to_sqlitedb_model(query)?;
        }
        db.execute(query).await?;
        Ok(())
    }

    async fn sqlitedb_delete(db: &SqliteDb, collection_id: &Uuid, id: &Uuid) -> Result<()> {
        let mut column = HashSet::<String>::with_capacity(1);
        column.insert("_id".to_owned());
        db.execute(
            sqlx::query(&sqlite_record::delete(
                &Self::new_table_name(collection_id),
                &column,
            ))
            .bind(id),
        )
        .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct RecordFilters(Vec<RecordFilter>);

impl RecordFilters {
    pub fn new(data: &Vec<RecordFilter>) -> Self {
        Self(data.to_vec())
    }

    pub fn scylladb_filter_query(
        &self,
        logical_operator: &Option<&str>,
        level: usize,
    ) -> Result<String> {
        if level > 1 {
            return Err(Error::msg(
                "ScyllaDB doesn't support filter query with level greater than 2",
            ));
        }
        let mut filter = String::new();
        for (idx, f) in self.0.iter().enumerate() {
            if idx > 0 {
                if filter.len() > 0 {
                    filter += " ";
                }
                if let Some(operator) = *logical_operator {
                    filter += &format!("{operator} ");
                }
            }
            let op = f.op.to_uppercase();
            if let Some(child) = &f.child {
                if SCYLLA_LOGICAL_OPERATOR.contains(&op.as_str()) {
                    filter += &child.scylladb_filter_query(&Some(&op), level + 1)?;
                } else {
                    return Err(Error::msg(format!(
                        "Operator '{op}' is not supported as a logical operator in ScyllaDB"
                    )));
                }
            } else {
                let field = f.field.as_ref().unwrap();
                if SCYLLA_COMPARISON_OPERATOR.contains(&op.as_str()) {
                    filter += &format!("\"{}\" {}", field, &op);
                    if f.value.is_some() {
                        filter += " ?";
                    }
                } else {
                    return Err(Error::msg(format!(
                        "Operator '{op}' is not supported as a comparison operator in ScyllaDB"
                    )));
                }
            }
        }
        Ok(filter)
    }

    pub fn scylladb_values(&self) -> Result<Vec<Box<dyn SerializeCql>>> {
        let mut values = Vec::with_capacity(self.values_capacity());
        for f in &self.0 {
            if let Some(value) = &f.value {
                values.push(value.to_scylladb_model()?)
            }
            if let Some(child) = &f.child {
                values.append(&mut child.scylladb_values()?)
            }
        }
        Ok(values)
    }

    pub fn postgresdb_filter_query(
        &self,
        logical_operator: &Option<&str>,
        level: usize,
        first_argument_idx: &mut usize,
    ) -> Result<String> {
        let mut filter = String::new();
        if level > 1 {
            filter += "("
        }
        for (idx, f) in self.0.iter().enumerate() {
            if idx > 0 {
                if filter.len() > 0 {
                    filter += " ";
                }
                if let Some(operator) = *logical_operator {
                    filter += &format!("{operator} ");
                }
            }
            let op = f.op.to_uppercase();
            if let Some(child) = &f.child {
                if POSTGRES_LOGICAL_OPERATOR.contains(&op.as_str()) {
                    filter += &child.postgresdb_filter_query(
                        &Some(&op),
                        level + 1,
                        first_argument_idx,
                    )?;
                } else {
                    return Err(Error::msg(format!(
                        "Operator '{op}' is not supported as a logical operator in PostgreSQL"
                    )));
                }
            } else {
                if POSTGRES_COMPARISON_OPERATOR.contains(&op.as_str()) {
                    filter += &format!("\"{}\" {}", f.field.as_ref().unwrap(), &op);
                    if f.value.is_some() {
                        filter += &format!(" ${}", first_argument_idx);
                        *first_argument_idx += 1;
                    }
                } else {
                    return Err(Error::msg(format!(
                        "Operator '{op}' is not supported as a comparison operator in PostgreSQL"
                    )));
                }
            }
        }
        if level > 1 {
            filter += ")"
        }
        Ok(filter)
    }

    pub fn postgresdb_values<'a>(
        &self,
        query: sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments>,
    ) -> Result<sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments>> {
        let mut query = query;
        for f in &self.0 {
            if let Some(value) = &f.value {
                query = value.to_postgresdb_model(query)?
            }
            if let Some(child) = &f.child {
                query = child.postgresdb_values(query)?
            }
        }
        Ok(query)
    }

    pub fn postgresdb_values_as<'a, T>(
        &self,
        query: sqlx::query::QueryAs<'a, sqlx::Postgres, T, sqlx::postgres::PgArguments>,
    ) -> Result<sqlx::query::QueryAs<'a, sqlx::Postgres, T, sqlx::postgres::PgArguments>> {
        let mut query = query;
        for f in &self.0 {
            if let Some(value) = &f.value {
                query = value.to_postgresdb_model_as(query)?
            }
            if let Some(child) = &f.child {
                query = child.postgresdb_values_as(query)?
            }
        }
        Ok(query)
    }

    pub fn mysqldb_filter_query(
        &self,
        logical_operator: &Option<&str>,
        level: usize,
    ) -> Result<String> {
        let mut filter = String::new();
        if level > 1 {
            filter += "("
        }
        for (idx, f) in self.0.iter().enumerate() {
            if idx > 0 {
                if filter.len() > 0 {
                    filter += " ";
                }
                if let Some(operator) = *logical_operator {
                    filter += &format!("{operator} ");
                }
            }
            let op = f.op.to_uppercase();
            if let Some(child) = &f.child {
                if MYSQL_LOGICAL_OPERATOR.contains(&op.as_str()) {
                    filter += &child.mysqldb_filter_query(&Some(&op), level + 1)?;
                } else {
                    return Err(Error::msg(format!(
                        "Operator '{op}' is not supported as a logical operator in MySQL"
                    )));
                }
            } else {
                if MYSQL_COMPARISON_OPERATOR.contains(&op.as_str()) {
                    filter += &format!("`{}` {}", f.field.as_ref().unwrap(), &op);
                    if f.value.is_some() {
                        filter += " ?";
                    }
                } else {
                    return Err(Error::msg(format!(
                        "Operator '{op}' is not supported as a comparison operator in MySQL"
                    )));
                }
            }
        }
        if level > 1 {
            filter += ")"
        }
        Ok(filter)
    }

    pub fn mysqldb_values<'a>(
        &self,
        query: sqlx::query::Query<'a, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    ) -> Result<sqlx::query::Query<'a, sqlx::MySql, sqlx::mysql::MySqlArguments>> {
        let mut query = query;
        for f in &self.0 {
            if let Some(value) = &f.value {
                query = value.to_mysqldb_model(query)?
            }
            if let Some(child) = &f.child {
                query = child.mysqldb_values(query)?
            }
        }
        Ok(query)
    }

    pub fn mysqldb_values_as<'a, T>(
        &self,
        query: sqlx::query::QueryAs<'a, sqlx::MySql, T, sqlx::mysql::MySqlArguments>,
    ) -> Result<sqlx::query::QueryAs<'a, sqlx::MySql, T, sqlx::mysql::MySqlArguments>> {
        let mut query = query;
        for f in &self.0 {
            if let Some(value) = &f.value {
                query = value.to_mysqldb_model_as(query)?
            }
            if let Some(child) = &f.child {
                query = child.mysqldb_values_as(query)?
            }
        }
        Ok(query)
    }

    pub fn sqlitedb_filter_query(
        &self,
        logical_operator: &Option<&str>,
        level: usize,
    ) -> Result<String> {
        let mut filter = String::new();
        if level > 1 {
            filter += "("
        }
        for (idx, f) in self.0.iter().enumerate() {
            if idx > 0 {
                if filter.len() > 0 {
                    filter += " ";
                }
                if let Some(operator) = *logical_operator {
                    filter += &format!("{operator} ");
                }
            }
            let op = f.op.to_uppercase();
            if let Some(child) = &f.child {
                if SQLITE_LOGICAL_OPERATOR.contains(&op.as_str()) {
                    filter += &child.sqlitedb_filter_query(&Some(&op), level + 1)?;
                } else {
                    return Err(Error::msg(format!(
                        "Operator '{op}' is not supported as a logical operator in SQLite"
                    )));
                }
            } else {
                if SQLITE_COMPARISON_OPERATOR.contains(&op.as_str()) {
                    filter += &format!("`{}` {}", f.field.as_ref().unwrap(), &op);
                    if f.value.is_some() {
                        filter += " ?";
                    }
                } else {
                    return Err(Error::msg(format!(
                        "Operator '{op}' is not supported as a comparison operator in SQLite"
                    )));
                }
            }
        }
        if level > 1 {
            filter += ")"
        }
        Ok(filter)
    }

    pub fn sqlitedb_values<'a>(
        &self,
        query: sqlx::query::Query<'a, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<sqlx::query::Query<'a, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'a>>> {
        let mut query = query;
        for f in &self.0 {
            if let Some(value) = &f.value {
                query = value.to_sqlitedb_model(query)?
            }
            if let Some(child) = &f.child {
                query = child.sqlitedb_values(query)?
            }
        }
        Ok(query)
    }

    pub fn sqlitedb_values_as<'a, T>(
        &self,
        query: sqlx::query::QueryAs<'a, sqlx::Sqlite, T, sqlx::sqlite::SqliteArguments<'a>>,
    ) -> Result<sqlx::query::QueryAs<'a, sqlx::Sqlite, T, sqlx::sqlite::SqliteArguments<'a>>> {
        let mut query = query;
        for f in &self.0 {
            if let Some(value) = &f.value {
                query = value.to_sqlitedb_model_as(query)?
            }
            if let Some(child) = &f.child {
                query = child.sqlitedb_values_as(query)?
            }
        }
        Ok(query)
    }

    fn values_capacity(&self) -> usize {
        let mut capacity = self.0.len();
        for f in &self.0 {
            if let Some(child) = &f.child {
                capacity += child.values_capacity()
            }
        }
        capacity
    }
}

#[derive(Clone)]
pub struct RecordFilter {
    field: Option<String>,
    op: String,
    value: Option<ColumnValue>,
    child: Option<RecordFilters>,
}

impl RecordFilter {
    pub fn new(
        field: &Option<String>,
        op: &str,
        value: &Option<ColumnValue>,
        child: &Option<RecordFilters>,
    ) -> Self {
        Self {
            field: field.to_owned(),
            op: op.to_owned(),
            value: value.clone(),
            child: child.clone(),
        }
    }

    pub fn field(&self) -> &Option<String> {
        &self.field
    }

    pub fn op(&self) -> &str {
        &self.op
    }

    pub fn value(&self) -> &Option<ColumnValue> {
        &self.value
    }

    pub fn child(&self) -> &Option<RecordFilters> {
        &self.child
    }
}

pub struct RecordOrder {
    field: String,
    kind: String,
}

impl RecordOrder {
    pub fn new(field: &str, kind: &str) -> Self {
        Self {
            field: field.to_owned(),
            kind: kind.to_owned(),
        }
    }

    pub fn field(&self) -> &str {
        &self.field
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }
}

pub struct RecordPagination {
    limit: Option<i32>,
}

impl RecordPagination {
    pub fn new(limit: &Option<i32>) -> Self {
        Self { limit: *limit }
    }

    pub fn limit(&self) -> &Option<i32> {
        &self.limit
    }
}
