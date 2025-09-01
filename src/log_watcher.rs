// src/log_watcher.rs
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::thread;

use notify::{
    Config as NotifyConfig, Event, Error as NotifyError, RecommendedWatcher, RecursiveMode, Watcher,
};

/// This function monitors a folder for `.log` files using file system notifications.
/// It prints the content of newly created or modified `.log` files, and tracks the last
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
/// 1. Reads the initial content of all `.log` files within the folder upon starting.
/// 2. Listens for file system events, such as creation, modification, or deletion of `.log` files.
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

    // Maps each file path to its last read offset by storing its byte position
    let mut positions: HashMap<PathBuf, u64> = HashMap::new();

    // Read the initial content of all log files
    if let Ok(entries) = std::fs::read_dir(&folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("log") {
                match File::open(&path) {
                    Ok(mut f) => {
                        let mut contents = String::new();
                        if f.read_to_string(&mut contents).is_ok() && !contents.is_empty() {
                            println!("--- {} (initial) ---\n{}", path.display(), contents);
                        }
                        if let Ok(pos) = f.seek(SeekFrom::Current(0)) {
                            positions.insert(path.clone(), pos);
                        }
                    }
                    Err(err) => eprintln!("Failed to open {}: {}", path.display(), err),
                }
            }
        }
    }
    
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
                            if let Err(e) = read_new(path, &mut positions) {
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
fn read_new(path: &PathBuf, positions: &mut HashMap<PathBuf, u64>) -> std::io::Result<()> {
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

    // Read the new content
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;

    // Display the new content to stdout if it's not empty
    if !buf.is_empty() {
        println!("--- {} (appended) ---\n{}", path.display(), buf);
    }

    // Update the position in the positions map with the new seek position after reading
    let new_pos = f.seek(SeekFrom::Current(0))?;
    positions.insert(path.clone(), new_pos);
    Ok(())
}