pub mod models;
pub mod repository_default;
pub mod repository_servers;
use crate::config::Config;

// Expose Database type under `db::repository::Database`
pub mod repository {
    pub use super::repository_default::Database;
}

// Initializes the Database from the environment variable
pub fn init_database_from_env() -> Result<repository::Database, Box<dyn std::error::Error>> {
    let cfg = Config::from_env()?;
    let db = repository::Database::new(&cfg.database_url)?;
    // db.ping()?; // Fail if credentials/host are wrong
    Ok(db)
}
