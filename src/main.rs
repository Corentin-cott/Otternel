mod config; // Import config module
mod db;
mod serverlog;
mod helper;
mod playerstats;

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
    let container = "creatif";  // Nom du conteneur temp
    let world = "netherhub";    // Nom du monde temp

    match playerstats::minecraft_players::fetch_mc_player_stats(container, world).await {
        Ok(stats) => {
            for (uuid, json) in stats {
                trace!("Joueur {} : {:?}", uuid, json);
            }
        }
        Err(e) => {
            error!("Erreur lors de la récupération des stats : {}", e);
        }
    }
}