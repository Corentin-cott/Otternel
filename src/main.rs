mod config;
mod db;
mod serverlog;
mod helper;
mod playerstats;

use tokio::task;
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
    task::spawn_blocking(move || { // Use spawn_blocking and a normal closure
        if let Err(err) = serverlog::log_watcher::watch_serverlogs(&log_folder) {
            error!("Log watcher failed: {}", err);
        }
    });

    // Check if player stats fetch is enabled
    if !cfg.get_player_stats_enabled {
        info!("{}", "Periodic playerstats fetch is DISABLED (GET_PLAYER_STATS_ENABLED is false)".yellow());
        future::pending::<()>().await; // Even if player stats is disabled, keep the program running
    }

    info!("{}", "GET_PLAYER_STATS_ENABLED is true : periodic playerstats fetch is starting...".green().bold());

    let every_sec = cfg.periodic_events_every_sec;
    info!("{}", format!("Periodic stats fetch will run every {} seconds", every_sec).cyan());

    // Create the periodic interval
    let mut interval = tokio::time::interval(Duration::from_secs(every_sec));

    loop {
        interval.tick().await;
        info!("{}", "Triggering periodic playerstats fetch...".blue());
        periodic_playerstats_fetch().await;
        info!("{}", "Periodic playerstats fetch cycle completed. Waiting for next interval.".black().bold());
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

    // Minecraft player stats fetch ended, send embed
    if let Err(e) = send_discord_embed(
        "otternel",
        "",
        "Enregistrement des stats de joueurs Minecraft",
        "",
        "Enregistrement des stats de joueurs Minecraft terminé.",
        Some("126020".to_string()),
        "",
        "",
        "",
        "Otternel Service",
        Some(chrono::Utc::now().to_rfc3339()),
    ) {
        error!("{e}");
    }
}
