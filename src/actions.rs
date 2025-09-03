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
        "on_player_joined" => on_player_joined(line, serverlog_id),
        "on_player_left" => on_player_left(line, serverlog_id),
        "on_test" => on_test(line, serverlog_id),
        _ => eprintln!("Unknown action function: {}", function),
    }
}

// Actions
fn on_test(line: &str, serverlog_id: u32) {
    println!("[action] on_test triggered with serverlog_id={} line: {}", serverlog_id, line);
}

fn on_player_joined(line: &str, serverlog_id: u32) {
    println!("[action] on_player_joined triggered with serverlog_id={} line: {}", serverlog_id, line);

    if let Err(e) = crate::services::webhook_discord::send_discord_embed(
        "otternel",
        &format!("on_player_joined (log {}): {}", serverlog_id, line),
        "Player joined",
        "https://antredesloutres.fr/",
        0x000000,
        "",
    ) {
        eprintln!("[action] {e}");
    }
}

fn on_player_left(line: &str, serverlog_id: u32) {
    println!("[action] on_player_left triggered with serverlog_id={} line: {}", serverlog_id, line);
}