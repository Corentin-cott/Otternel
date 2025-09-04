use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::thread;
use regex::Regex;
use serde::Deserialize;

use notify::{
    Config as NotifyConfig, Event, Error as NotifyError, RecommendedWatcher, RecursiveMode, Watcher,
};
use crate::serverlog;

/// Decodes a sequence of bytes into a `String`, attempting to interpret the input as UTF-8 or UTF-16 with a fallback mechanism.
///
/// # Parameters
/// - `bytes`: A byte slice (`&[u8]`) representing the encoded data to decode.
///
/// # Returns
/// - A `String` containing the decoded text. If the input bytes are empty, it returns an empty string. If decoding fails, a lossy UTF-8 representation of the input bytes is returned.
///
/// # Behavior
/// 1. If the input `bytes` is empty, an empty string is returned.
/// 2. If the input can successfully be decoded as UTF-8, that string is returned.
/// 3. If decoding as UTF-8 fails, the function attempts to decode as UTF-16:
///    - If the first two bytes match the UTF-16 little-endian byte order mark (BOM, `0xFFFE`), they are skipped.
///    - If the length of the slice is odd (not divisible by 2), the last trailing byte is removed to align pairs of bytes for UTF-16 decoding.
///    - The byte pairs are interpreted as UTF-16 little-endian code units, converted into a `String`.
/// 4. If the UTF-16 decoding fails, the function finally falls back to a lossy UTF-8 representation of the input bytes for the output.
///
fn decode_log_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    let mut slice = bytes;
    if slice.len() >= 2 && slice[0] == 0xFF && slice[1] == 0xFE {
        slice = &slice[2..];
    }
    if slice.len() < 2 {
        return String::new();
    }
    if slice.len() % 2 != 0 {
        slice = &slice[..slice.len() - 1];
    }
    let code_units: Vec<u16> = slice
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16(&code_units).unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string())
}

/// This function monitors a folder for `.log` files using file system notifications.
/// It prints the content of newly created or modified `.log` files and tracks the last
/// read position in the file to ensure only new additions are read subsequently. Deleted
/// `.log` files are also handled by removing them from the internal tracking state.
///
/// # Arguments
///
/// * `folder` - A string slice representing the path to the folder that should be watched for changes.
///
/// # Returns
///
/// Returns a `Result`:
/// - `Ok(())` if the folder was successfully watched.
/// - `Err(NotifyError)` if an error occurs during setup or watch.
///
/// # Behavior
///
/// 1. Loads the triggers from the `triggers.toml` file.
/// 2. Reads the initial content of all `.log` files within the folder upon starting.
/// 3. Listens for file system events, such as creation, modification, or deletion of `.log` files.
/// - For created or modified `.log` files, it prints the new content appended to the files.
/// - Removes deleted `.log` files from the tracking state.
/// - Handles errors, such as unable to read a file or watcher errors, and retries the watcher.
///
/// # Errors
///
/// This function may return the following errors:
///
/// - If the specified folder does not exist, it returns a generic `NotifyError`.
/// - Any errors inherent to `notify` library operations, such as watcher setup or event handling, are returned.
///
pub fn watch_serverlogs(folder: &str) -> Result<(), NotifyError> {
    let folder = PathBuf::from(folder);
    if !folder.exists() { // We check that the folder exists
        return Err(NotifyError::generic(&format!("Folder {} does not exist", folder.display())));
    }

    // Load the triggers from the triggers.toml file at the root of the project
    #[derive(Deserialize)]
    struct Trigger { name: Option<String>, pattern: String, function: String, serverlog_ids: Option<Vec<u32>> }
    #[derive(Deserialize)]
    struct TriggerFile { trigger: Vec<Trigger> }

    let compiled_triggers: Vec<(Regex, String, Option<Vec<u32>>)> = (|| {
        let content = std::fs::read_to_string("triggers.toml").ok()?;
        let tf: TriggerFile = toml::from_str(&content).ok()?;
        let mut out = Vec::new();
        for t in tf.trigger {
            match Regex::new(&t.pattern) {
                Ok(re) => out.push((re, t.function, t.serverlog_ids)),
                Err(e) => eprintln!(
                    "Invalid regex in trigger '{}': {} ({})",
                    t.name.unwrap_or_default(),
                    t.pattern,
                    e
                ),
            }
        }
        Some(out)
    })().unwrap_or_else(|| {
        eprintln!("No triggers loaded (missing or invalid triggers.toml)");
        Vec::new()
    });
    println!("Loaded {} triggers", compiled_triggers.len());

    // Maps each file path to its last read offset by storing its byte position
    let mut positions: HashMap<PathBuf, u64> = HashMap::new();
    
    // Create the watcher and start watching the folder
    let (tx, rx) = channel::<Result<Event, NotifyError>>();
    let mut watcher: RecommendedWatcher = RecommendedWatcher::new(move |res| {
        // Ignore if the watcher thread panics
        let _ = tx.send(res);
    }, NotifyConfig::default())?;

    // Watch the folder for changes
    watcher.watch(&folder, RecursiveMode::NonRecursive)?;

    // Loop forever, reading new content of log files as they are appended
    println!("Watching folder {} for .log changes...", folder.display());
    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                for path in &event.paths { // For each file that changed...
                    if path.extension().and_then(|s| s.to_str()) != Some("log") { // ...if it's not a log file, ignore it
                        continue;
                    }

                    use notify::event::EventKind;

                    match &event.kind {
                        // When a .log file is created or modified, we read its new content
                        EventKind::Create(_) | EventKind::Modify(_) => {
                            if let Err(e) = read_new(path, &mut positions, &compiled_triggers) {
                                eprintln!("Error reading {}: {}", path.display(), e);
                            }
                        }
                        // When a .log file is removed, we remove it from the position map
                        EventKind::Remove(_) => {
                            positions.remove(path);
                            println!("File removed: {}", path.display());
                        }
                        _ => {}
                    }
                }
            }
            // The file was read, but an error occurred
            Ok(Err(e)) => eprintln!("Watcher error: {}", e),
            // The file could not be read
            Err(e) => {
                eprintln!("Watcher channel receive error: {}", e);
                const WAIT_TIME: u64 = 1;
                println!("Retrying in {} second...", WAIT_TIME);
                thread::sleep(std::time::Duration::from_secs(WAIT_TIME));
            }
        }
    }
}

/// Reads the newly appended content from a file starting from the last known position.
/// If the file has been truncated or rotated, it will read from the beginning of the file.
///
/// # Arguments
/// - `path`: A `PathBuf` reference representing the path of the file to read from.
/// - `positions`: A mutable reference to a `HashMap` that tracks the last read position of each file.
///   The key is the `PathBuf` of the file, and the value is the last read byte position (`u64`).
///
/// # Returns
/// Returns a `Result`:
/// - `Ok(())`: On success.
/// - `Err(std::io::Error)`: If there is an error during file operations such as opening, seeking, or reading.
///
/// # Behavior
/// 1. Opens the file at the specified `path`.
/// 2. Retrieves the metadata of the file, including its size.
/// 3. Uses the last known position from the `positions` map to determine where to start reading:
///     - If the file's length is less than the last known position, it assumes the file was
///       truncated or rotated, resets the position to the start of the file, and logs this to `stderr`.
/// 4. Seeks to the determined position in the file and reads the content from there.
/// 5. Displays the newly appended content (if any) to `stdout` prefixed with metadata.
/// 6. Updates the position in the `positions` map with the new seek position after reading.
///
/// # Notes
/// - The function assumes that the file may be appended over time and reads any new content since the last recorded position.
/// - Handles log rotation or truncation scenarios by resetting the read position to the start of the file.
///
fn read_new(path: &PathBuf, positions: &mut HashMap<PathBuf, u64>, compiled_triggers: &[(Regex, String, Option<Vec<u32>>)]) -> std::io::Result<()> {
    let mut f = File::open(path)?;
    let metadata = f.metadata()?;
    let len = metadata.len();
    let last = positions.get(path).cloned().unwrap_or(0);

    // Check if the file was truncated or rotated
    if len < last {
        // Truncated or rotated; reset the position to the start of the file
        eprintln!("File {} was truncated/rotated; reading from start", path.display());
        positions.insert(path.clone(), 0);
    }

    // Seek to the last known position and read the new content
    let start = positions.get(path).cloned().unwrap_or(0);
    f.seek(SeekFrom::Start(start))?;

    // Read the new content (bytes)
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes)?;
    let buf = decode_log_bytes(&bytes);

    // Only proceed if there is any new text
    if !buf.is_empty() {
        // Get serverlog_id from file name once
        let serverlog_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.parse::<u32>().ok());

        // Decide which line to use:
        // - If chunk ends with newline => use last line.
        // - Else => use the previous line (last complete).
        let ends_with_newline = buf.ends_with('\n') || buf.ends_with("\r\n");
        let mut lines = buf.lines().rev();

        let last_line = if ends_with_newline {
            lines.next()
        } else {
            lines.nth(1) // skip the trailing partial, take previous
        };

        if let (Some(id), Some(line)) = (serverlog_id, last_line) {
            // Optional: print only the last line for visibility
            println!("--- {} (last line) ---\n{}", path.display(), line);

            // Match triggers only against the last (complete) line
            for (re, func, ids_opt) in compiled_triggers {
                if re.is_match(line) {
                    if ids_opt.as_ref().map(|ids| ids.contains(&id)).unwrap_or(true) {
                        serverlog::actions::dispatch(func, line, id);
                    }
                }
            }
        }
    }

    // Then keep your existing position update:
    let new_pos = f.seek(SeekFrom::Current(0))?;
    positions.insert(path.clone(), new_pos);

    Ok(())
}