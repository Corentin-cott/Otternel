use colored::Colorize;
use log::{debug, error, info, warn};
use crate::{helper};
use crate::db::models::{PlayerConnectionLog, Server};

/// Dispatches a function call based on the input function name. Logs an error message if no function matches.
///
/// # Arguments
///
/// * `function` - A string slice that contains the name of the function to dispatch.
/// * `line` - A string slice passed as an argument to the matched function.
/// * `serverlog_id` - The `active_servers` record id, derived from the log folder name.
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
pub fn dispatch(function: &str, line: &str, serverlog_id: &str) {
    match function {
        "on_test" => on_test(serverlog_id),
        "on_player_message" => on_player_message(line, serverlog_id),
        "on_player_joined" => on_player_connection_update(line, serverlog_id, "rejoint"),
        "on_player_left" => on_player_connection_update(line, serverlog_id, "quitté"),
        "on_minecraft_player_advancement" => on_minecraft_player_advancement(line, serverlog_id),
        "on_player_death" => on_player_death(line, serverlog_id),
        "on_palworld_player_joined" => on_palworld_player_connection_update(line, serverlog_id, "rejoint"),
        "on_palworld_player_left" => on_palworld_player_connection_update(line, serverlog_id, "quitté"),
        _ => warn!("Unknown action function: {}", function.yellow()),
    }
}

// Actions
fn on_test(serverlog_id: &str) {
    info!("{} triggered with serverlog_id={}", "on_test".green().bold(), serverlog_id.green().bold());
}

fn on_player_connection_update(line: &str, serverlog_id: &str, co_type: &str) {
    // Resolve active server at serverlog_id
    let server: Server = get_server_by_id(serverlog_id);

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
        if let Err(e) = helper::code_generator::handle_unlinked_player_join(&db, &player_id, playername, serverlog_id) {
            error!("Failed to process player join for '{}': {}", playername, e);
        }
    }

    // We log the player connection in database
    let log = PlayerConnectionLog {
        server: server.id.clone(),
        player: player_id.clone(),
        logged_at: chrono::Utc::now().to_rfc3339(),
    };
    if let Err(e) = db.insert_player_connection_log(&log) {
        warn!("Failed to insert player connection log: {:?}", e);
    }
    if let Err(e) = db.update_last_connection(&player_id) {
        warn!("Failed to update last player connection: {:?}", e);
    }

    // Send Discord embed with the player's name
    if let Err(e) = helper::webhook_discord::send_discord_embed(
        helper::webhook_discord::get_webhook_identity_by_server_id(server.platform_name.as_deref().unwrap_or("")),
        " ",
        playername,
        &format!("https://antredesloutres.fr/joueurs/minecraft/{}", playername.to_lowercase()),
        &format!("{playername} a {co_type} {}", server.name),
        server.embed_color.clone(),
        " ",
        " ",
        " ",
        &format!("Message de {}", server.name),
        Some(chrono::Utc::now()
            .to_rfc3339())
    ) {
        error!("{e}");
    }

    // Broadcast connection/disconnection to other servers
    let playername = playername.to_string();
    let server_nom = server.name.clone();
    let co_type = co_type.to_string();
    let origin_id = serverlog_id.to_string();

    tokio::spawn(async move {
        let rcon = match helper::rcon_helper::RconHelper::new() {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to init RconHelper for cross-server broadcast: {}", e);
                return;
            }
        };

        let servers = match rcon.db.get_all_active_global_servers() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to fetch active global servers: {}", e);
                return;
            }
        };

        let command = format!(
            r#"tellraw @a [{{"text":"{} a {} {} !","color":"yellow"}}]"#,
            playername,
            co_type,
            server_nom
        );

        for target_server in servers.into_iter().filter(|s| s.server_id != origin_id) {
            match rcon.execute_command(&target_server.server_id, &command).await {
                Ok(_) => debug!(
                    "Cross-server connection update relayed to server id={}",
                    target_server.server_id
                ),
                Err(e) => warn!(
                    "Failed to relay connection update to server id={}: {}",
                    target_server.server_id, e
                ),
            }
        }
    });
}

fn on_palworld_player_connection_update(line: &str, serverlog_id: &str, co_type: &str) {
    // Extract playername, platform id and character id from a line like:
    // "[2026-10-23 22:58:47] [LOG] playername joined the server. (User id: steam_7656..., Player id: 3C36EC75...)"
    // The "Player id" part may be missing (left lines, older server versions)
    let re = regex::Regex::new(
        r"(?:.*\[LOG\])?\s*(.+?) (?:joined|left) the server\.? \(User id: ([^,)]+)(?:, Player id: ([0-9A-Fa-f]+))?\)"
    ).unwrap();
    let Some(caps) = re.captures(line) else {
        debug!("no palworld connection match: {}", line);
        return;
    };
    let playername = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("Joueur");
    let account_id = caps.get(2).map(|m| m.as_str().trim()).unwrap_or_default().to_string();
    let player_uid = caps.get(3).map(|m| m.as_str().to_uppercase());

    // Resolve active server at serverlog_id
    let server: Server = get_server_by_id(serverlog_id);

    // Load configuration for DB pool before logging player connection
    let db = match helper::open_database::open_db_from_env() {
        Some(db) => db,
        None => {
            warn!("Could not load DB configuration to resolve active server");
            return;
        }
    };

    // We need to get the player id. If the player isn't in the database, they will be added
    let player_id = match db.add_palworld_player_if_not_exist(&account_id, playername) {
        Ok(id) => id,
        Err(err) => {
            error!("Player {}'s ID couldn't be fetched or added to the database: {}", playername, err);
            return;
        }
    };

    // Saves don't contain the platform id: link the player to their character on join.
    // Exact match on the character id when the line has it, else on the name
    if co_type == "rejoint" {
        let linked = match &player_uid {
            Some(uid) => db.link_palworld_player(&server.id, uid, &player_id, playername).map(|_| true),
            None => db.link_palworld_player_by_name(&server.id, playername, &player_id),
        };
        match linked {
            Ok(true) => info!("Palworld player {} ({}) linked to their character", playername.green().bold(), account_id),
            Ok(false) => debug!("No unlinked palworld character named {} on server {}", playername, server.name),
            Err(e) => warn!("Failed to link palworld player {}: {:?}", playername, e),
        }
    }

    // We log the player connection in database
    let log = PlayerConnectionLog {
        server: server.id.clone(),
        player: player_id.clone(),
        logged_at: chrono::Utc::now().to_rfc3339(),
    };
    if let Err(e) = db.insert_player_connection_log(&log) {
        warn!("Failed to insert player connection log: {:?}", e);
    }
    if let Err(e) = db.update_last_connection(&player_id) {
        warn!("Failed to update last player connection: {:?}", e);
    }

    // Send Discord embed with the player's name
    if let Err(e) = helper::webhook_discord::send_discord_embed(
        helper::webhook_discord::get_webhook_identity_by_server_id(server.platform_name.as_deref().unwrap_or("")),
        " ",
        playername,
        "",
        &format!("{playername} a {co_type} {}", server.name),
        server.embed_color.clone(),
        " ",
        " ",
        " ",
        &format!("Message de {}", server.name),
        Some(chrono::Utc::now().to_rfc3339())
    ) {
        error!("{e}");
    }
}

fn on_player_message(line: &str, serverlog_id: &str) {
    // Resolve active server at serverlog_id
    let server: Server = get_server_by_id(serverlog_id);

    let re = regex::Regex::new(r"<([^>]+)>\s(.+)").unwrap();
    let caps = re.captures(line).unwrap();

    let playername = caps.get(1).unwrap().as_str();
    let message = caps.get(2).unwrap().as_str();
    let embed_color = server.embed_color.clone().unwrap_or_else(|| "white".to_string());

    // Send Discord embed with the player's message
    if let Err(e) = helper::webhook_discord::send_discord_embed(
        helper::webhook_discord::get_webhook_identity_by_server_id(server.platform_name.as_deref().unwrap_or("")),
        " ",
        playername,
        &format!("https://antredesloutres.fr/joueurs/minecraft/{}", playername.to_lowercase()),
        message,
        server.embed_color.clone(),
        &format!("https://mc-heads.net/avatar/{}/50", playername.to_lowercase()),
        " ",
        " ",
        &format!("Message de {}", server.name),
        Some(chrono::Utc::now().to_rfc3339()),
    ) {
        error!("{e}");
    }

    // Send the message to the players in other servers (except the one it comes from)
    let playername = playername.to_string();
    let message = message.to_string();
    let origin_id = serverlog_id.to_string();

    tokio::spawn(async move {
        let rcon = match helper::rcon_helper::RconHelper::new() {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to init RconHelper for cross-server broadcast: {}", e);
                return;
            }
        };

        let servers = match rcon.db.get_all_active_global_servers() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to fetch active global servers: {}", e);
                return;
            }
        };

        let command = format!(
            r#"tellraw @a [{{"text":"<"}},{{"text":"{}","color":"{}"}},{{"text":"> {}","color":"white"}}]"#,
            playername,
            embed_color,
            message
        );

        for target_server in servers.into_iter().filter(|s| s.server_id != origin_id) {
            match rcon.execute_command(&target_server.server_id, &command).await {
                Ok(_) => debug!(
                    "Cross-server message relayed to server id={}",
                    target_server.server_id
                ),
                Err(e) => warn!(
                    "Failed to relay message to server id={}: {}",
                    target_server.server_id, e
                ),
            }
        }
    });
}

fn on_minecraft_player_advancement(line: &str, serverlog_id: &str) {
    // Resolve active server at serverlog_id
    let server: Server = get_server_by_id(serverlog_id);

    let re = regex::Regex::new(
        r"^(?:\[[^\]]+\]\s*:?\s*)*([^ ]+)\s+(?:has made the advancement|completed the challenge|reached the goal)\s+\[?(.+?)\]$"
    ).unwrap();

    if let Some(caps) = re.captures(line) {
        let playername = caps.get(1).map(|m| m.as_str()).unwrap_or("Joueur");
        let advancement = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        // Send Discord embed with the player's message
        if let Err(e) = helper::webhook_discord::send_discord_embed(
        helper::webhook_discord::get_webhook_identity_by_server_id(server.platform_name.as_deref().unwrap_or("")),
            " ",
            playername,
            &format!("https://antredesloutres.fr/joueurs/minecraft/{}", playername.to_lowercase()),
            &format!("{} a obtenu l'avancement {} sur {} !", playername, advancement, server.name),
            server.embed_color.clone(),
            " ",
            " ",
            " ",
            &format!("Message de {}", server.name),
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

fn on_player_death(line: &str, serverlog_id: &str) {

    if line.contains("l='ServerLevel") || line.contains(", message:") {
        debug!("Ignoring non-player death log line: {}", line);
        return;
    }

    // Resolve active server from serverlog_id
    let server: Server = get_server_by_id(serverlog_id);

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
        helper::webhook_discord::get_webhook_identity_by_server_id(server.platform_name.as_deref().unwrap_or("")),
        " ",
        &format!("{playername} est mort sur {} !", server.name),
        &format!("https://antredesloutres.fr/joueurs/minecraft/{}", playername.to_lowercase()),
        &format!("{playername} {death_message}"),
        server.embed_color.clone(),
        " ",
        " ",
        " ",
        &format!("Message de {}", server.name),
        Some(chrono::Utc::now().to_rfc3339()),
    ) {
        error!("{e}");
    }
}

fn get_server_by_id(serverlog_id: &str) -> Server {
    // Load configuration for DB pool
    let db = match helper::open_database::open_db_from_env() {
        Some(db) => db,
        None => {
            warn!("Could not load DB configuration to resolve active server");
            return Server::default();
        }
    };

    match db.get_server_by_id(serverlog_id) {
        Ok(Some(s)) => {
            debug!(
                "Resolved active server {} -> '{}'",
                serverlog_id.green().bold(),
                s.name.green().bold()
            );
            s
        }
        Ok(None) => {
            error!("No server found for active server id {}", serverlog_id);
            Server::default()
        }
        Err(e) => {
            error!(
                "Error fetching server for active server id {}: {}",
                serverlog_id, e
            );
            Server::default()
        }
    }
}
