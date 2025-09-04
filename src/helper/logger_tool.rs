use chrono::Local;
use colored::*;

pub(crate) fn setup_logger() -> Result<(), fern::InitError> {
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
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}