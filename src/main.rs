mod config; // Import config module
mod db;
mod serverlog;
mod helper;

use log::{info, error};

/**
Entry point of Otternel
*/
fn main() {
    // Splash screen, it's not useful, but it's cool
    println!("\n
*  _____ _   _                   _
| |     | |_| |_ ___ ___ ___ ___| |
| |  |  |  _|  _| -_|  _|   | -_| |
| |_____|_| |_| |___|_| |_|_|___|_|
└ {}\n", chrono::Local::now()
    );

    // Try to load configuration from environment variables
    match config::Config::from_env() {
        Ok(cfg) => {
            // Load the logging tool
            helper::logger_tool::setup_logger(&cfg.log_level)
                .expect("Failed to initialize logger");

            info!("Config loaded successfully");

            // Start the watcher — the function is blocking and runs indefinitely
            if let Err(err) = serverlog::log_watcher::watch_serverlogs(&cfg.serverlog_folder) {
                error!("Log watcher failed: {}", err);
            }
        }
        Err(err) => {
            // Initialize a minimal logger to be able to log the error
            helper::logger_tool::setup_logger("warn")
                .ok();
            error!("Failed to load config: {}", err);
        }
    }
}
