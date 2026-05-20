use serde::Deserialize;

// Each field corresponds to one environment variable
#[derive(Deserialize, Debug)]
pub struct Config {
    pub log_level: String,
    pub database_url: String,
    pub serverlog_folder: String,
    pub otternel_webhook_activated: String,
    pub otternel_webhook_url: String,
    pub mineotter_bot_webhook_activated: String,
    pub mineotter_bot_webhook_url: String,
    pub multiloutre_bot_webhook_activated: String,
    pub multiloutre_bot_webhook_url: String,
    pub mcmyadmin_webhook_activated: String,
    pub mcmyadmin_webhook_url: String,
    pub mcmyadmin_secondary_webhook_activated: String,
    pub mcmyadmin_secondary_webhook_url: String,

    #[serde(default = "default_get_player_stats_enabled")]
    pub get_player_stats_enabled: bool,

    #[serde(default = "default_periodic_events_every_sec")]
    pub periodic_events_every_sec: u64,

    pub periodic_events_every_min: Option<u64>,
}

fn default_get_player_stats_enabled() -> bool {
    false
}

fn default_periodic_events_every_sec() -> u64 {
    3600
}

impl Config {
    /// This function loads the .env file and deserializes the environment variables into a Config struct
    pub fn from_env() -> Result<Self, envy::Error> {
        // Load variables from a `.env` file if present
        dotenvy::dotenv().ok();
        envy::from_env()
    }
}
