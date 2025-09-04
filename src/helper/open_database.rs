/// Opens a database connection based on configuration loaded from environment variables.
///
/// This function attempts to load the application configuration using
/// `crate::config::Config::from_env`. If the configuration is successfully loaded, it retrieves
/// the database URL and tries to initialize a database connection using
/// `crate::db::repository_default::Database::new`. If either step fails, an error message is logged
/// to standard error, and `None` is returned.
///
/// # Returns
///
/// * `Some(Database)` - If the configuration is successfully loaded, and a database connection
///   is successfully created.
/// * `None` - If either the configuration cannot be loaded, or the database connection fails.
///
/// # Error Handling
///
/// - Logs an error and returns `None` if the configuration cannot be loaded.
/// - Logs an error with details and returns `None` if the database connection fails.
///
/// # Example
///
/// ```rust
/// if let Some(db) = open_db_from_env() {
///     println!("Database connection established successfully.");
/// } else {
///     eprintln!("Failed to establish database connection.");
/// }
/// ```
pub fn open_db_from_env() -> Option<crate::db::repository_default::Database> {
    let cfg = match crate::config::Config::from_env() {
        Ok(cfg) => cfg,
        Err(_) => {
            eprintln!("[action] could not load configuration to resolve active server");
            return None;
        }
    };
    match crate::db::repository_default::Database::new(&cfg.database_url) {
        Ok(db) => Some(db),
        Err(e) => {
            eprintln!("[action] could not create DB pool: {:?}", e);
            None
        }
    }
}