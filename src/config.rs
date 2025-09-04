use serde::Deserialize;

// Each field corresponds to one environment variable
#[derive(Deserialize, Debug)]
pub struct Config {
    pub database_url: String,
    pub serverlog_folder: String,
    pub otternel_webhook_activated: String,
    pub otternel_webhook_url: String,
    pub mineotter_bot_webhook_activated: String,
    pub mineotter_bot_webhook_url: String,
    pub multiloutre_bot_webhook_activated: String,
    pub multiloutre_bot_webhook_url: String,
}

impl Config {
    /// This function loads the .env file and deserializes the environment variables into a Config struct
    pub fn from_env() -> Result<Self, envy::Error> {
        // Load variables from a `.env` file if present
        dotenvy::dotenv().ok();
        envy::from_env()
    }
}
