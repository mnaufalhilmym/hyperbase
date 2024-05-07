use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(FromRow)]
pub struct RemoteSyncModel {
    remote_address: String,
    remote_id: Uuid,
    last_data_sync: DateTime<Utc>,
    last_change_id: Uuid,
}

impl RemoteSyncModel {
    pub fn new(
        remote_address: &str,
        remote_id: &Uuid,
        last_data_sync: &DateTime<Utc>,
        last_change_id: &Uuid,
    ) -> Self {
        Self {
            remote_address: remote_address.to_owned(),
            remote_id: *remote_id,
            last_data_sync: *last_data_sync,
            last_change_id: *last_change_id,
        }
    }

    pub fn remote_address(&self) -> &str {
        &self.remote_address
    }

    pub fn remote_id(&self) -> &Uuid {
        &self.remote_id
    }

    pub fn last_data_sync(&self) -> &DateTime<Utc> {
        &self.last_data_sync
    }

    pub fn last_change_id(&self) -> &Uuid {
        &self.last_change_id
    }
}
