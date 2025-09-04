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

    // Load the logging tool
    helper::logger_tool::setup_logger().expect("Failed to initialize logger");

    // Try to load configuration from environment variables
    match config::Config::from_env() {
        Ok(cfg) => {
            info!("Config loaded successfully");
            
            // Start the watcher — the function is blocking and runs indefinitely
            if let Err(err) = serverlog::log_watcher::watch_serverlogs(&cfg.serverlog_folder) {
                error!("Log watcher failed: {}", err);
            }
        }
        Err(err) => error!("Failed to load config: {}", err),
    }
}
