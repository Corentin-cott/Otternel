
mod config; // Import config module

/**
Entry point of Otternel
*/
fn main() {
    // Try to load configuration from environment variables
    match config::Config::from_env() {
        // Ok = If successful
        Ok(cfg) => {
            println!("Config loaded successfully:");
            println!("DATABASE_URL: {}", cfg.database_url);
            println!("SERVERLOG_FOLDER: {}", cfg.serverlog_folder);
        }
        // Err = If failed
        Err(err) => {
            eprintln!("Failed to load config: {}", err);
        }
    }
}
