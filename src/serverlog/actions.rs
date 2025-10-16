use colored::Colorize;
use log::{debug, error, info, warn};
use crate::{helper};
use crate::db::models::{JoueurConnectionLog, Serveur};

/// Dispatches a function call based on the input function name. Logs an error message if no function matches.
///
/// # Arguments
///
/// * `function` - A string slice that contains the name of the function to dispatch.
/// * `line` - A string slice passed as an argument to the matched function.
/// * `serverlog_id` - The numeric identifier of the server log file, derived from its file name.
///
///  # Returns
/// This function does not return any value. It either executes the matched function
/// or prints an error message to stderr.
///
/// # Behavior
///
/// - If `function` is `"on_player_joined"`, it calls `on_player_joined(line, serverlog_id)`.
/// - If `function` is `"on_player_left"`, it calls `on_player_left(line, serverlog_id)`.
/// - etc...
/// - If `function` does not match any of the above cases, it logs an error
///   message to the standard error output.
///
pub fn dispatch(function: &str, line: &str, serverlog_id: u32) {
    match function {
        "on_test" => on_test(serverlog_id),
        "on_player_message" => on_player_message(line, serverlog_id),
        "on_player_joined" => on_player_connection_update(line, serverlog_id, "rejoint"),
        "on_player_left" => on_player_connection_update(line, serverlog_id, "quitté"),
        "on_minecraft_player_advancement" => on_minecraft_player_advancement(line, serverlog_id),
        "on_player_death" => on_player_death(line, serverlog_id),
        _ => warn!("Unknown action function: {}", function.yellow()),
    }
}

// Actions
fn on_test(serverlog_id: u32) {
    info!("{} triggered with serverlog_id={}", "on_test".green().bold(), serverlog_id.to_string().green().bold());
}

fn on_player_connection_update(line: &str, serverlog_id: u32, co_type: &str) {
    // Resolve active server at serverlog_id
    let server:Serveur = get_server_by_active_server_id(serverlog_id);

    // Extract playername from a line like:
    // "[00:00:000] [Server thread/INFO]: playername joined/left the game"
    let playername = line
        .split("]: ")
        .nth(1)
        .and_then(|s| {
            s.strip_suffix(" left the game")
                .or_else(|| s.strip_suffix(" joined the game"))
        })
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("Joueur");

    // Load configuration for DB pool before logging player connection
    let db = match helper::open_database::open_db_from_env() {
        Some(db) => db,
        None => {
            warn!("Could not load DB configuration to resolve active server");
            return;
        }
    };

    // We need to get the player id. If the player isn't in the database, they will be added
    let player_id = match db.add_and_get_minecraft_player_id(playername) {
        Ok(id) => id, // Successfully retrieved the player ID
        Err(err) => {
            error!("Player {}'s ID couldn't be fetched or added to the database: {}", playername, err);
            return; // or handle the error appropriately
        }
    };

    // We check if the player's account is link & if `co_type` = rejoint. If not, we generate a code to link it
    if co_type == "rejoint" {
        match db.is_account_linked_to_user(player_id) {
            Ok(is_linked) => {
                if !is_linked {
                    // TODO : CHECK IF A CODE AS ALREADY ACTIVE
                    // 1. The account is not linked, generate a new code.
                    let nouveau_code = helper::code_generator::generer_code_unique();
                    info!(
                        "Player '{}' is not linked. Generating code: {}",
                        playername, nouveau_code
                    );

                    // 2. Save the code to the database with a 10-minute expiration.
                    match db.creer_code_liaison(player_id, &nouveau_code, 10) {
                        Ok(_) => {
                            info!("Successfully saved linking code for player '{}'.", playername);
                            // TODO : SEND THE CODE TO THE PLAYER
                        }
                        Err(e) => {
                            error!(
                                "Failed to save linking code for player '{}': {}",
                                playername, e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                error!(
                    "Could not check if player '{}' is linked: {}",
                    playername, e
                );
            }
        }
    }
    // =======================================================

    // We log the player connection in database
    let log = JoueurConnectionLog {
        serveur_id: server.id,
        joueur_id: player_id,
        date: chrono::Utc::now().naive_utc(),
    };
    if let Err(e) = db.insert_joueur_connection_log(&log) {
        warn!("Failed to insert player connection log: {:?}", e);
    }

    // Send Discord embed with the player's name
    if let Err(e) = helper::webhook_discord::send_discord_embed(
        helper::webhook_discord::get_webhook_identity_by_server_id(server.jeu),
        " ",
        playername,
        &format!("https://antredesloutres.fr/joueurs/minecraft/{}", playername.to_lowercase()),
        &format!("{playername} a {co_type} {}", server.nom),
        server.embed_color,
        " ",
        " ",
        " ",
        &format!("Message de {}", server.nom),
        Some(chrono::Utc::now()
            .to_rfc3339())
    ) {
        error!("{e}");
    }
}

fn on_player_message(line: &str, serverlog_id: u32) {
    // Resolve active server at serverlog_id
    let server:Serveur = get_server_by_active_server_id(serverlog_id);

    let re = regex::Regex::new(r"<([^>]+)>\s(.+)").unwrap();
    let caps = re.captures(line).unwrap();

    let playername = caps.get(1).unwrap().as_str();
    let message = caps.get(2).unwrap().as_str();

    // Send Discord embed with the player's message
    if let Err(e) = helper::webhook_discord::send_discord_embed(
        helper::webhook_discord::get_webhook_identity_by_server_id(server.jeu),
        " ",
        playername,
        &format!("https://antredesloutres.fr/joueurs/minecraft/{}", playername.to_lowercase()),
        message,
        server.embed_color,
        &format!("https://mc-heads.net/avatar/{}/50", playername.to_lowercase()),
        " ",
        " ",
        &format!("Message de {}", server.nom),
        Some(chrono::Utc::now()
            .to_rfc3339())
    ) {
        error!("{e}");
    }
}

fn on_minecraft_player_advancement(line: &str, serverlog_id: u32) {
    // Resolve active server at serverlog_id
    let server:Serveur = get_server_by_active_server_id(serverlog_id);

    let re = regex::Regex::new(
        r"^(?:\[[^\]]+\]\s*:?\s*)*([^ ]+)\s+(?:has made the advancement|completed the challenge|reached the goal)\s+\[?(.+?)\]$"
    ).unwrap();

    if let Some(caps) = re.captures(line) {
        let playername = caps.get(1).map(|m| m.as_str()).unwrap_or("Joueur");
        let advancement = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        // Send Discord embed with the player's message
        if let Err(e) = helper::webhook_discord::send_discord_embed(
        helper::webhook_discord::get_webhook_identity_by_server_id(server.jeu),
            " ",
            playername,
            &format!("https://antredesloutres.fr/joueurs/minecraft/{}", playername.to_lowercase()),
            &format!("{} a obtenu l'avancement {} sur {} !", playername, advancement, server.nom),
            server.embed_color,
            " ",
            " ",
            " ",
            &format!("Message de {}", server.nom),
            Some(chrono::Utc::now()
                .to_rfc3339())
        ) {
            error!("{e}");
        }
    } else {
        // Avoid panic
        debug!("no advancement match: {}", line);
    }
}

fn on_player_death(line: &str, serverlog_id: u32) {
    // Resolve active server from serverlog_id
    let server: Serveur = get_server_by_active_server_id(serverlog_id);

    // Exemple de ligne : "[17:58:38] [Server thread/INFO]: TheAzertor fell from a high place"
    let re = regex::Regex::new(r": ([^ ]+) (.+)$").unwrap();
    let (playername, death_message) = if let Some(caps) = re.captures(line) {
        (
            caps.get(1).map(|m| m.as_str()).unwrap_or("Joueur"),
            caps.get(2).map(|m| m.as_str()).unwrap_or("est mort."),
        )
    } else {
        ("Joueur", "est mort.")
    };

    // Envoi de l'embed Discord
    if let Err(e) = helper::webhook_discord::send_discord_embed(
        helper::webhook_discord::get_webhook_identity_by_server_id(server.jeu),
        " ",
        &format!("{playername} est mort sur {} !", server.nom),
        &format!("https://antredesloutres.fr/joueurs/minecraft/{}", playername.to_lowercase()),
        &format!("{playername} {death_message}"),
        server.embed_color,
        " ",
        " ",
        " ",
        &format!("Message de {}", server.nom),
        Some(chrono::Utc::now().to_rfc3339()),
    ) {
        error!("{e}");
    }
}

fn get_server_by_active_server_id(serverlog_id: u32) -> Serveur {
    // Load configuration for DB pool
    let db = match helper::open_database::open_db_from_env() {
        Some(db) => db,
        None => {
            warn!("Could not load DB configuration to resolve active server");
            return Serveur::default();
        }
    };

    match db.get_server_by_active_server_id(serverlog_id as u64) {
        Ok(Some(s)) => {
            debug!(
                "Resolved active server {} -> '{}'",
                serverlog_id.to_string().green().bold(),
                s.nom.green().bold()
            );
            s
        }
        Ok(None) => {
            error!("No server found for active server id {}", serverlog_id);
            Serveur::default()
        }
        Err(e) => {
            error!(
                "Error fetching server for active server id {}: {}",
                serverlog_id, e
            );
            Serveur::default()
        }
    }
}
