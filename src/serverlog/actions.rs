use crate::helper;

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
        "on_test" => on_test(line, serverlog_id),
        "on_player_message" => on_player_message(line, serverlog_id),
        "on_player_joined" => on_player_connection_update(line, serverlog_id, "rejoint"),
        "on_player_left" => on_player_connection_update(line, serverlog_id, "quitté"),
        _ => eprintln!("Unknown action function: {}", function),
    }
}

// Actions
fn on_test(line: &str, serverlog_id: u32) {
    println!("[action] on_test triggered with serverlog_id={} line: {}", serverlog_id, line);
}

fn on_player_connection_update(line: &str, serverlog_id: u32, co_type: &str) {
    // Load configuration for DB pool
    let db = match helper::open_database::open_db_from_env() {
        Some(db) => db,
        None => {
            eprintln!("[error] could not load configuration to resolve active server");
            return;
        }
    };

    // Resolve active server at serverlog_id
    let server = match db.get_server_by_active_server_id(serverlog_id as u64) {
        Ok(Some(s)) => {
            println!("[action] resolved active server {} -> '{}'", serverlog_id, s.nom);
            s
        }
        Ok(None) => {
            eprintln!("[action] no server found for active server id {}", serverlog_id);
            return;
        }
        Err(e) => {
            eprintln!("[action] error fetching server for active server id {}: {}", serverlog_id, e);
            return;
        }
    };

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
        .unwrap_or("Un joueur");

    // Send Discord embed with the player's name
    if let Err(e) = helper::webhook_discord::send_discord_embed(
        "otternel",
        "",
        playername,
        &format!("https://antredesloutres.fr/joueurs/minecraft/{}", playername.to_lowercase()),
        &format!("{playername} a {co_type} {}", server.nom),
        server.embed_color,
        "",
        "",
        "",
        &format!("{}", server.nom),
        Some(chrono::Utc::now()
            .to_rfc3339())
    ) {
        eprintln!("[action] {e}");
    }
}

fn on_player_message(line: &str, serverlog_id: u32) {
    // Load configuration for DB pool
    let cfg = match crate::config::Config::from_env() {
        Ok(cfg) => cfg,
        Err(_) => {
            eprintln!("[action] could not load configuration to resolve active server");
            return;
        }
    };

    // Create DB pool
    let db = match crate::db::repository_default::Database::new(&cfg.database_url) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("[action] could not create DB pool: {:?}", e);
            return;
        }
    };

    // Resolve active server at serverlog_id
    let server = match db.get_server_by_active_server_id(serverlog_id as u64) {
        Ok(Some(s)) => {
            println!("[action] resolved active server {} -> '{}'", serverlog_id, s.nom);
            s
        }
        Ok(None) => {
            eprintln!("[action] no server found for active server id {}", serverlog_id);
            return;
        }
        Err(e) => {
            eprintln!("[action] error fetching server for active server id {}: {}", serverlog_id, e);
            return;
        }
    };

    // TODO : Extrat player name from player message in line, then send it in embed

    // Temporary placeholder values
    let playername = "Un joueur";
    let message = "Un message";

    // Send Discord embed with the player's message
    if let Err(e) = crate::helper::webhook_discord::send_discord_embed(
        "otternel",
        message,
        playername,
        &format!("https://antredesloutres.fr/joueurs/minecraft/{}", playername.to_lowercase()),
        &format!("on_player_message (log {})", serverlog_id),
        server.embed_color,
        &format!("https://mc-heads.net/avatar/{}", playername.to_lowercase()),
        "",
        "",
        &format!("Message de {}", server.nom),
        None
    ) {
        eprintln!("[action] {e}");
    }
}