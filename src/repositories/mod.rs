pub mod crawl;
pub mod credential;
pub mod files;
pub mod invoice;
pub mod link;
pub mod user;

pub fn is_fk_violation(e: &sqlx::Error) -> bool {
    matches!(
        e,
        sqlx::Error::Database(db) if db.code().as_deref() == Some("23503")
    )
}
