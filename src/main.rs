mod config;
mod db;
mod serverlog;
mod helper;
mod playerstats;
use futures::future;

use colored::Colorize;
use log::{info, error};
use std::time::Duration;

/**
Entry point of Otternel
*/
#[tokio::main]
async fn main() {
    // Splash screen, it's not useful, but it's cool
    println!(
        "
*  _____ _   _                   _
| |     | |_| |_ ___ ___ ___ ___| |
| |  |  |  _|  _| -_|  _|   | -_| |
| |_____|_| |_| |___|_| |_|_|___|_|
└ {}\n",
        chrono::Local::now()
    );

    // Try to load configuration from environment variables
    let cfg = match config::Config::from_env() {
        Ok(c) => c,
        Err(err) => {
            helper::logger_tool::setup_logger("warn").ok();
            error!("Failed to load config: {}", err);
            return;
        }
    };

    // Load the logging tool
    helper::logger_tool::setup_logger(&cfg.log_level)
        .expect("Failed to initialize logger");
    info!("Config loaded successfully");

    // Start the watcher
    let log_folder = cfg.serverlog_folder.clone();
    tokio::spawn(async move {
        if let Err(err) = serverlog::log_watcher::watch_serverlogs(&log_folder) {
            error!("Log watcher failed: {}", err);
        }
    });

    // Check if player stats fetch is enabled
    let get_player_stats_enabled = std::env::var("GET_PLAYER_STATS_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase() == "true";

    if !get_player_stats_enabled {
        info!("{}", "GET_PLAYER_STATS_ENABLED is false : periodic events will not start".yellow());
        future::pending::<()>().await; // Even if player stats is disabled, keep the program running
    }

    // Read the interval from environment
    let every_sec: u64 = std::env::var("PERIODIC_EVENTS_EVERY_SEC")
        .expect("PERIODIC_EVENTS_EVERY_SEC must be set in the environment")
        .parse()
        .expect("PERIODIC_EVENTS_EVERY_SEC must be a valid number");

    info!("{}", format!("Periodic event launching every {} seconds", every_sec).green());

    // Create the periodic interval
    let mut interval = tokio::time::interval(Duration::from_secs(every_sec));

    loop {
        interval.tick().await;
        periodic_playerstats_fetch().await;
    }

}

async fn periodic_playerstats_fetch() {
    use helper::webhook_discord::send_discord_embed;

    // Send embed
    if let Err(e) = send_discord_embed(
        "otternel",
        "",
        "Enregistrement des stats de joueurs Minecraft",
        "",
        "Passage sur chaque serveur de la table `serveurs`.",
        Some("126020".to_string()),
        "",
        "",
        "",
        "Otternel Service",
        Some(chrono::Utc::now().to_rfc3339()),
    ) {
        error!("{e}");
    }

    // Launch minecraft player stats
    if let Err(e) = playerstats::minecraft_players::sync_mc_stats_to_db().await {
        error!("Erreur sync_mc_stats_to_db: {e:?}");
    }
}
