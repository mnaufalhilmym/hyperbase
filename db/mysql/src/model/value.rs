use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy)]
pub enum ColumnKind {
    Tinyint1,
    Boolean,
    Bool,
    Tinyint,
    Smallint,
    Int,
    Bigint,
    TinyintUnsigned,
    SmallintUnsigned,
    IntUnsigned,
    BigintUnsigned,
    Float,
    Double,
    Varchar,
    Char,
    Text,
    Varbinary,
    Binary,
    Blob,
    Timestamp,
    Datetime,
    Date,
    Time,
    Decimal,
    Binary16,
    Char36,
    Char32,
    Json,
}

impl ColumnKind {
    pub fn to_str(&self) -> &str {
        match self {
            Self::Tinyint1 => "tinyint(1)",
            Self::Boolean => "boolean",
            Self::Bool => "bool",
            Self::Tinyint => "tinyint",
            Self::Smallint => "smallint",
            Self::Int => "int",
            Self::Bigint => "bigint",
            Self::TinyintUnsigned => "tinyint unsigned",
            Self::SmallintUnsigned => "smallint unsigned",
            Self::IntUnsigned => "int unsigned",
            Self::BigintUnsigned => "bigint unsigned",
            Self::Float => "float",
            Self::Double => "double",
            Self::Varchar => "varchar(16383)",
            Self::Char => "char",
            Self::Text => "text",
            Self::Varbinary => "varbinary",
            Self::Binary => "binary",
            Self::Blob => "blob",
            Self::Timestamp => "timestamp",
            Self::Datetime => "datetime",
            Self::Date => "date",
            Self::Time => "time",
            Self::Decimal => "decimal",
            Self::Binary16 => "binary(16)",
            Self::Char36 => "char(36)",
            Self::Char32 => "char(32)",
            Self::Json => "json",
        }
    }

    pub fn from_str(str: &str) -> Result<Self, String> {
        match str {
            "tinyint(1)" => Ok(Self::Tinyint1),
            "boolean" => Ok(Self::Boolean),
            "bool" => Ok(Self::Bool),
            "tinyint" => Ok(Self::Tinyint),
            "smallint" => Ok(Self::Smallint),
            "int" => Ok(Self::Int),
            "bigint" => Ok(Self::Bigint),
            "tinyint unsigned" => Ok(Self::TinyintUnsigned),
            "smallint unsigned" => Ok(Self::SmallintUnsigned),
            "int unsigned" => Ok(Self::IntUnsigned),
            "bigint unsigned" => Ok(Self::BigintUnsigned),
            "float" => Ok(Self::Float),
            "double" => Ok(Self::Double),
            "varchar(16383)" => Ok(Self::Varchar),
            "char" => Ok(Self::Char),
            "text" => Ok(Self::Text),
            "varbinary" => Ok(Self::Varbinary),
            "binary" => Ok(Self::Binary),
            "blob" => Ok(Self::Blob),
            "timestamp" => Ok(Self::Timestamp),
            "datetime" => Ok(Self::Datetime),
            "date" => Ok(Self::Date),
            "time" => Ok(Self::Time),
            "decimal" => Ok(Self::Decimal),
            "binary(16)" => Ok(Self::Binary16),
            "char(36)" => Ok(Self::Char36),
            "char(32)" => Ok(Self::Char32),
            "json" => Ok(Self::Json),
            _ => Err(format!("Unknown schema field kind '{str}'")),
        }
    }
}