mod config; // Import config module
mod db;
mod serverlog;
mod helper;
mod playerstats;

use colored::Colorize;
use log::{info, error, trace};

/**
Entry point of Otternel
*/
fn main() {
    // Splash screen, it's not useful, but it's cool
    println!("
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

            // Load the periodic tasks
            periodic_playerstats_fetch()

            /* Disable the watcher for now

            // Start the watcher — the function is blocking and runs indefinitely
            if let Err(err) = serverlog::log_watcher::watch_serverlogs(&cfg.serverlog_folder) {
                error!("Log watcher failed: {}", err);
            }
            */
        }
        Err(err) => {
            // Initialize a minimal logger to be able to log the error
            helper::logger_tool::setup_logger("warn")
                .ok();
            error!("Failed to load config: {}", err);
        }
    }
}

#[tokio::main]
async fn periodic_playerstats_fetch() {
    info!("{} {} {}", "Starting periodic playerstats fetch for".blue().bold(), "Minecraft".green().bold(), "players :".blue().bold());
    if let Err(e) = playerstats::minecraft_players::sync_mc_stats_to_db().await {
        error!("Erreur sync_mc_stats_to_db: {e:?}");
    }
}