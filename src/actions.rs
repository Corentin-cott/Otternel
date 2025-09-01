/// Dispatches a function call based on the input function name. Logs an error message if no function matches.
///
/// # Arguments
///
/// * `function` - A string slice that contains the name of the function to dispatch.
/// * `line` - A string slice passed as an argument to the matched function.
///
///  # Returns
/// This function does not return any value. It either executes the matched function
/// or prints an error message to stderr.
///
/// # Behavior
///
/// - If `function` is `"on_player_joined"`, it calls `on_player_joined(line)`.
/// - If `function` is `"on_player_left"`, it calls `on_player_left(line)`.
/// - If `function` is `"on_test"`, it calls `on_test(line)`.
/// - If `function` does not match any of the above cases, it logs an error
///   message to the standard error output.
///
pub fn dispatch(function: &str, line: &str) {
    match function {
        "on_player_joined" => on_player_joined(line),
        "on_player_left" => on_player_left(line),
        "on_test" => on_test(line),
        _ => eprintln!("Unknown action function: {}", function),
    }
}

// Actions

pub fn on_player_joined(line: &str) {
    println!("[action] on_player_joined triggered with line: {}", line);
}

pub fn on_player_left(line: &str) {
    println!("[action] on_player_left triggered with line: {}", line);
}

pub fn on_test(line: &str) {
    println!("[action] on_test triggered with line: {}", line);
}