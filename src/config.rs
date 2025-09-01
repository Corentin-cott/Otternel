// We use `serde` to easily map environment variables into a Rust struct
use serde::Deserialize;

// Each field corresponds to one environment variable
#[derive(Deserialize, Debug)]
pub struct Config {
    pub serverlog_folder: String,
}

impl Config {
    // This function loads the .env file and deserializes the environment variables into a Config struct
    pub fn from_env() -> Result<Self, envy::Error> {
        // Load variables from a `.env` file if present
        dotenvy::dotenv().ok();

        // Use `envy` to deserialize environment variables into the Config struct. It automatically matches variable names to struct fields
        envy::from_env()
    }
}
