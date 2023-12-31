use scylla::CachingSession;

pub const INSERT: &str = "INSERT INTO \"hyperbase\".\"collections\" (\"id\", \"created_at\", \"updated_at\", \"project_id\", \"name\", \"schema_fields\", \"indexes\") VALUES (?, ?, ?, ?, ?, ?, ?)";
pub const SELECT: &str = "SELECT \"id\", \"created_at\", \"updated_at\", \"project_id\", \"name\", \"schema_fields\", \"indexes\" FROM \"hyperbase\".\"collections\" WHERE \"id\" = ?";
pub const SELECT_MANY_BY_PROJECT_ID: &str = "SELECT \"id\", \"created_at\", \"updated_at\", \"project_id\", \"name\", \"schema_fields\", \"indexes\" FROM \"hyperbase\".\"collections\" WHERE \"project_id\" = ?";
pub const UPDATE: &str = "UPDATE \"hyperbase\".\"collections\" SET \"updated_at\" = ?, \"name\" = ?, \"schema_fields\" = ?, \"indexes\" = ? WHERE \"id\" = ?";
pub const DELETE: &str = "DELETE FROM \"hyperbase\".\"collections\" WHERE \"id\" = ?";

pub async fn init(cached_session: &CachingSession) {
    hb_log::info(Some("🔧"), "ScyllaDB: Setting up collections table");

    cached_session
        .get_session()
        .query(
            "CREATE TYPE IF NOT EXISTS \"hyperbase\".\"schema_field_props\" (\"kind\" text, \"internal_kind\" text, \"required\" boolean)",
            &[],
        )
        .await
        .unwrap();
    cached_session.get_session().query("CREATE TABLE IF NOT EXISTS \"hyperbase\".\"collections\" (\"id\" uuid, \"created_at\" timestamp, \"updated_at\" timestamp, \"project_id\" uuid, \"name\" text, \"schema_fields\" map<text, frozen<schema_field_props>>, \"indexes\" set<text>, PRIMARY KEY (\"id\"))", &[]).await.unwrap();
    cached_session
        .get_session()
        .query(
            "CREATE INDEX IF NOT EXISTS ON \"hyperbase\".\"collections\" (\"project_id\")",
            &[],
        )
        .await
        .unwrap();

    cached_session
        .add_prepared_statement(&INSERT.into())
        .await
        .unwrap();
    cached_session
        .add_prepared_statement(&SELECT.into())
        .await
        .unwrap();
    cached_session
        .add_prepared_statement(&SELECT_MANY_BY_PROJECT_ID.into())
        .await
        .unwrap();
    cached_session
        .add_prepared_statement(&UPDATE.into())
        .await
        .unwrap();
    cached_session
        .add_prepared_statement(&DELETE.into())
        .await
        .unwrap();
}
