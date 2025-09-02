mod config; // Import config module
mod log_watcher;
mod services;
mod actions;

/**
Entry point of Otternel
*/
fn main() {
    // Splash screen, it's not useful, but it's cool
    println!("
*       _____   __    __                              __
|      /     \\_/  |__/  |_  ___________  ____   ____ |  |
|     /   |   \\   __\\   __\\/ __ \\_  __ \\/    \\_/ __ \\|  |
|    /    |    \\  |  |  | \\  ___/|  | \\/   |  \\  ___/|  |__
|    \\_______  /__|  |__|  \\___  >__|  |___|  /\\___  >____/
*            \\/                \\/           \\/     \\/\
    ");


    // Try to load configuration from environment variables
    match config::Config::from_env() {
        Ok(cfg) => {
            println!("Config loaded successfully: {}", cfg.serverlog_folder);
            
            // Start the watcher — the function is blocking and runs indefinitely
            if let Err(err) = log_watcher::watch_serverlogs(&cfg.serverlog_folder) {
                eprintln!("Log watcher failed: {}", err);
            }
        }
        Err(err) => eprintln!("Failed to load config: {}", err),
    }
}
