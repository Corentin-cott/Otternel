use chrono::Local;
use colored::*;
use log::LevelFilter;

pub(crate) fn setup_logger(log_level_str: &str) -> Result<(), fern::InitError> {
    let level_filter = match log_level_str.to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info"  => LevelFilter::Info,
        "warn"  => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Warn,
    };

    fern::Dispatch::new()
        .format(|out, message, record| {
            let level = match record.level() {
                log::Level::Error => "ERROR".red().bold(),
                log::Level::Warn  => "WARN.".yellow().bold(),
                log::Level::Info  => "INFO.".green(),
                log::Level::Debug => "DEBUG".blue(),
                log::Level::Trace => "TRACE".cyan(),
            };

            out.finish(format_args!(
                "[{}][{}][{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                level,
                record.target(),
                message
            ))
        })
        .level(level_filter)
        .level_for("ureq", LevelFilter::Warn)
        .level_for("rustls", LevelFilter::Warn)
        .chain(std::io::stdout())
        .apply()?;

    Ok(())
}
