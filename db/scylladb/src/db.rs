use scylla::{
    serialize::row::SerializeRow,
    transport::{errors::QueryError, iterator::RowIterator},
    Bytes, CachingSession, QueryResult, SessionBuilder,
};

use crate::query::{
    admin, admin_password_reset, bucket, collection, file, keyspace, project, registration, token,
};

pub struct ScyllaDb {
    cached_session: CachingSession,
}

impl ScyllaDb {
    pub async fn new(
        user: &str,
        password: &str,
        host: &str,
        port: &str,
        replication_factor: &i64,
        cache_size: &usize,
        table_registration_ttl: &u32,
        table_reset_password_ttl: &u32,
    ) -> Self {
        hb_log::info(Some("⚡"), "ScyllaDB: Initializing component");

        let hostname = format!("{host}:{port}");
        let cached_session: CachingSession = CachingSession::from(
            SessionBuilder::new()
                .known_node(&hostname)
                .user(user, password)
                .build()
                .await
                .unwrap(),
            *cache_size,
        );

        Self::init(
            &cached_session,
            replication_factor,
            table_registration_ttl,
            table_reset_password_ttl,
        )
        .await;

        Self { cached_session }
    }

    pub async fn session_query(
        &self,
        query: &str,
        values: impl SerializeRow,
    ) -> Result<QueryResult, QueryError> {
        self.cached_session.get_session().query(query, values).await
    }

    pub async fn execute(
        &self,
        query: &str,
        values: impl SerializeRow,
    ) -> Result<QueryResult, QueryError> {
        self.cached_session.execute(query, values).await
    }

    pub async fn execute_iter(
        &self,
        query: &str,
        values: impl SerializeRow,
    ) -> Result<RowIterator, QueryError> {
        self.cached_session.execute_iter(query, values).await
    }

    pub async fn execute_paged(
        &self,
        query: &str,
        values: impl SerializeRow,
        paging_state: Option<Bytes>,
    ) -> Result<QueryResult, QueryError> {
        self.cached_session
            .execute_paged(query, values, paging_state)
            .await
    }

    async fn init(
        cached_session: &CachingSession,
        replication_factor: &i64,
        table_registration_ttl: &u32,
        table_reset_password_ttl: &u32,
    ) {
        // Create keyspace
        keyspace::init(cached_session, replication_factor).await;

        // Create tables
        admin::init(cached_session).await;
        token::init(cached_session).await;
        project::init(cached_session).await;
        collection::init(cached_session).await;
        bucket::init(cached_session).await;
        file::init(cached_session).await;
        registration::init(cached_session, table_registration_ttl).await;
        admin_password_reset::init(cached_session, table_reset_password_ttl).await;
    }
}
