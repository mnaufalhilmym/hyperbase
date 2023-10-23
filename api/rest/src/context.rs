use std::sync::mpsc::Sender;

use hb_db_scylladb::db::ScyllaDb;
use hb_hash_argon2::argon2::Argon2Hash;
use hb_mailer::MailPayload;

pub struct Context {
    pub hash: HashCtx,
    pub mailer: MailerCtx,
    pub db: DbCtx,
}

pub struct HashCtx {
    pub argon2: Argon2Hash,
}

pub struct MailerCtx {
    pub sender: Sender<MailPayload>,
}

pub struct DbCtx {
    pub scylladb: ScyllaDb,
}
