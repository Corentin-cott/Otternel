use log::{error, info, debug};
use crate::db::repository_default::Database;
use crate::helper::rcon_helper::RconHelper;
use crate::{helper};
use rand::{thread_rng, Rng};

// Charset without ambiguous characters (no I, O, 1, 0)
const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const CODE_LENGTH: usize = 9; /// Dosn't include dashes
const CHUNK_SIZE: usize = 3;

/// Generates a unique linking code.
/// This code is composed of 9 characters from a specific charset, formatted in chunks of 3 characters separated by dashes.
/// 
/// ## Returns
/// A `String` representing the generated linking code in the format "XXX-XXX-XXX".
///
pub fn generer_code_unique() -> String {
    let mut rng = thread_rng();
    let mut result = String::with_capacity(CODE_LENGTH + 2); // 9 chars + 2 tirets

    for i in 0..CODE_LENGTH {
        // Add a dash every CHUNK_SIZE characters (except at the start)
        if i > 0 && i % CHUNK_SIZE == 0 {
            result.push('-');
        }

        // Choose a random character from the charset
        let random_char = CHARSET[rng.gen_range(0..CHARSET.len())] as char;
        result.push(random_char);
    }

    result
}

/// Handles the scenario where a player joins a server and needs a linking code generated.
/// 
/// ## Arguments
/// * `db` - Reference to the database connection.
/// * `player_id` - The ID of the player joining the server.
/// * `playername` - The name of the player.
/// * `serverlog_id` - The ID of the server log.
/// 
/// ## Returns
/// * `Ok(())` - If the operation was successful
/// * `Result<(), Box<dyn std::error::Error>>` - Error otherwise
/// 
pub fn handle_unlinked_player_join(db: &Database, player_id: u64, playername: &str, serverlog_id: u32) -> Result<(), Box<dyn std::error::Error>> {
    // Checks if the generation of a code is necessary
    let is_linked = db.is_account_linked_to_user(player_id)?;
    if is_linked { return Ok(()); }

    let is_code_active = db.is_linking_code_active_for_player_id(player_id)?;
    if is_code_active { return Ok(()); }

    // Player is not linked and has no active code, proceed to generate one
    let new_code = helper::code_generator::generer_code_unique();
    info!("Player '{}' is not linked. Generating code: {}", playername, new_code);

    db.create_linking_code(player_id, &new_code, 10)?;
    debug!("Successfully saved linking code for player '{}'.", playername);

    // TODO : DEPENDING ON THE GAME OF THE PLAYER ID, CHANGE THE LOGIC
    // For now, we assume it's always Minecraft and we send the code via RCON
    let rcon_helper = RconHelper::new()?;
    let command_to_run = format!(
        r#"/tellraw {player} ["", {{"text":"[Antre des Loutres]","color":"gold"}}, {{"text":" Voici un code pour lié ton compte Minecraft à ton compte Discord : "}}, {{"text":"{code}","color":"gold"}}, {{"text":". Tu peux l'utiliser sur la page de ton profil "}}, {{"text":"(https://antredesloutres.fr)","italic":true,"underlined":true,"color":"dark_aqua","clickEvent":{{"action":"open_url","value":"https://antredesloutres.fr/joueurs/minecraft/{player_lc}/"}}}}, {{"text":"."}}]"#,
        player = playername,
        code = new_code,
        player_lc = playername.to_lowercase()
    ); 

    // Execute the RCON command asynchronously with Tokio
    tokio::spawn(async move {
        debug!("Sending RCON command to server ID {}", serverlog_id);
        if let Err(e) = rcon_helper.execute_command(serverlog_id as u64, &command_to_run).await {
            error!("Failed to execute RCON command: {}", e);
        }
    });
    
    Ok(())
}