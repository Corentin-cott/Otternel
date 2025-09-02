pub mod models;
pub mod repository_players_users;

use crate::config::Config;

// Initializes the Database from the environment variable
pub fn init_database_from_env() -> Result<repository::Database, Box<dyn std::error::Error>> {
    let cfg = Config::from_env()?;
    let db = repository::Database::new(&cfg.database_url)?;
    db.ping()?; // Fail if credentials/host are wrong
    Ok(db)
}