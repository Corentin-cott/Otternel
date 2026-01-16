use colored::*;
use log::{info, warn};

pub(crate) fn check_and_format_minecraft_uuid(player_uuid: &str) -> Result<String, fern::InitError> {
    info!(
        "{} {}",
        "Cheking player uuid:",
        player_uuid.yellow()
    );

    // dash format : 8-4-4-4-12
    const UUID_LEN_WITH_DASHES: usize = 36;
    const UUID_LEN_WITHOUT_DASHES: usize = 32;

    // Case : uuid without dashes
    if player_uuid.len() == UUID_LEN_WITHOUT_DASHES && player_uuid.chars().all(|c| c.is_ascii_hexdigit()) {
        warn!(
            "{} {}",
            "UUID is valid but missing dashes:".yellow(),
            player_uuid.red()
        );

        let formatted = format!(
            "{}-{}-{}-{}-{}",
            &player_uuid[0..8],
            &player_uuid[8..12],
            &player_uuid[12..16],
            &player_uuid[16..20],
            &player_uuid[20..32],
        );

        info!(
            "{} {}",
            "Expected format:".green(),
            formatted.bright_green()
        );

        return Ok(formatted);
    }

    // uuid strict validation
    if player_uuid.len() == UUID_LEN_WITH_DASHES {
        let dash_positions = [8, 13, 18, 23];

        let valid = player_uuid
            .chars()
            .enumerate()
            .all(|(i, c)| {
                if dash_positions.contains(&i) {
                    c == '-'
                } else {
                    c.is_ascii_hexdigit()
                }
            });

        if valid {
            info!(
                "{} {}",
                "UUID is valid:".green(),
                player_uuid.bright_green()
            );
            return Ok(player_uuid.to_string());
        } else {

            warn!(
                "{} {}",
                "UUID is invalid (wrong format):".red(),
                player_uuid.yellow()
            );
            return Err(fern::InitError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid Minecraft UUID",
            )));
        }
    }

    warn!(
        "{} {}",
        "UUID is invalid (incorrect length):".red(),
        player_uuid.yellow()
    );

    Err(fern::InitError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "Invalid Minecraft UUID",
    )))
}
