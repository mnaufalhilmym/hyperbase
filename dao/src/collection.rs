use chrono::{DateTime, Utc};
use hb_db_scylladb::model::collection::{
    CollectionScyllaModel, SchemaScyllaFieldKind, SchemaScyllaFieldModel,
};
use scylla::frame::value::Timestamp;
use uuid::Uuid;

use crate::util::conversion::datetime_to_duration_since_epoch;

pub struct CollectionModel {
    id: Uuid,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    project_id: Uuid,
    name: String,
    schema_fields: Vec<SchemaFieldModel>,
    indexes: Vec<String>,
}

impl CollectionModel {
    pub fn to_scylla_model(&self) -> CollectionScyllaModel {
        CollectionScyllaModel::new(
            self.id,
            Timestamp(datetime_to_duration_since_epoch(self.created_at)),
            Timestamp(datetime_to_duration_since_epoch(self.updated_at)),
            self.project_id,
            self.name.clone(),
            self.schema_fields
                .clone()
                .into_iter()
                .map(|schema_field| schema_field.to_scylla_model())
                .collect(),
            self.indexes.clone(),
        )
    }
}

#[derive(Clone)]
pub struct SchemaFieldModel {
    name: String,
    kind: SchemaFieldKind,
    required: bool,
}

impl SchemaFieldModel {
    pub fn to_scylla_model(self) -> SchemaScyllaFieldModel {
        SchemaScyllaFieldModel::new(self.name, self.kind.to_scylla_model(), self.required)
    }
}

#[derive(Clone)]
pub enum SchemaFieldKind {
    Boolean,
    Integer,
    Float,
    String,
    Uuid,
    Date,
    Time,
    Datetime,
    Timestamp,
}

impl SchemaFieldKind {
    pub fn to_scylla_model(&self) -> SchemaScyllaFieldKind {
        match self {
            Self::Boolean => SchemaScyllaFieldKind::Boolean,
            Self::Integer => SchemaScyllaFieldKind::Int,
            Self::Float => SchemaScyllaFieldKind::Double,
            Self::String => SchemaScyllaFieldKind::Text,
            Self::Uuid => SchemaScyllaFieldKind::Uuid,
            Self::Date => SchemaScyllaFieldKind::Date,
            Self::Time => SchemaScyllaFieldKind::Time,
            Self::Datetime => SchemaScyllaFieldKind::Timestamp,
            Self::Timestamp => SchemaScyllaFieldKind::Timestamp,
        }
    }
}