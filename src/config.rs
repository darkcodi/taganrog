use std::path::PathBuf;
use clap::ArgMatches;
use colored::Color;
use fern::colors::ColoredLevelConfig;
use home::home_dir;
use log::{error, info, LevelFilter};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub tg_homedir: PathBuf,
    pub db_filepath: PathBuf,
    pub thumbnails_dir: PathBuf,
}

pub fn configure_console_logging(matches: &ArgMatches) {
    let is_verbose = matches.get_one("verbose").map(|x: &bool| x.to_owned()).unwrap_or_default();
    let min_level = if is_verbose { LevelFilter::Debug } else { LevelFilter::Info };

    let stdout_config = fern::Dispatch::new()
        .format(|out, message, _| {
            out.finish(format_args!(
                "{}",
                message
            ))
        })
        .level(min_level)
        .filter(|metadata| metadata.level() != log::Level::Error)
        .chain(std::io::stdout());

    let stderr_config = fern::Dispatch::new()
        .level(LevelFilter::Error)
        .chain(std::io::stderr());

    fern::Dispatch::new()
        .chain(stdout_config)
        .chain(stderr_config)
        .apply()
        .expect("Failed to configure logging");
}

pub fn configure_api_logging(matches: &ArgMatches) {
    let is_verbose = matches.get_one("verbose").map(|x: &bool| x.to_owned()).unwrap_or_default();
    let min_level = if is_verbose { LevelFilter::Debug } else { LevelFilter::Info };

    let colors = ColoredLevelConfig::new()
        .trace(Color::Green)
        .debug(Color::Blue)
        .info(Color::BrightWhite)
        .warn(Color::Yellow)
        .error(Color::Red);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{green_color}{date}{clear_color} {gray_color}{level}{clear_color} {magenta_color}{target}{clear_color}] {level_color}{message}{clear_color}",
                date = humantime::format_rfc3339(std::time::SystemTime::now()),
                level = record.level(),
                target = record.target(),
                message = message,
                green_color = format_args!("\x1B[{}m", Color::Green.to_fg_str()),
                gray_color = format_args!("\x1B[{}m", Color::BrightBlack.to_fg_str()),
                magenta_color = format_args!("\x1B[{}m", Color::Magenta.to_fg_str()),
                level_color = format_args!("\x1B[{}m", colors.get_color(&record.level()).to_fg_str()),
                clear_color = "\x1B[0m",
            ))
        })
        .level(min_level)
        .level_for("tower_http::trace::on_response", LevelFilter::Debug)
        .level_for("tower_http::trace::make_span", LevelFilter::Debug)
        .chain(std::io::stdout())
        .apply()
        .expect("Failed to configure logging");
}

pub fn get_app_config_or_exit() -> AppConfig {
    let maybe_homedir_path = home_dir();
    if maybe_homedir_path.is_none() {
        error!("Homedir not found");
        std::process::exit(1);
    }
    let homedir_path = maybe_homedir_path.unwrap();

    let tg_homedir = homedir_path.join(".taganrog");
    if !tg_homedir.exists() {
        std::fs::create_dir(&tg_homedir).expect("Failed to create tg_homedir");
    }
    if tg_homedir.exists() && tg_homedir.is_file() {
        error!("tg_homedir is not a director: {:?}", tg_homedir);
        std::process::exit(1);
    }

    let db_filepath = tg_homedir.join("taganrog.db.json");
    if !db_filepath.exists() {
        std::fs::write(&db_filepath, "").expect("Failed to create db_filepath");
    }
    if db_filepath.exists() && db_filepath.is_dir() {
        error!("db_filepath is not a file: {:?}", db_filepath);
        std::process::exit(1);
    }

    let thumbnails_dir = tg_homedir.join("thumbnails");
    if !thumbnails_dir.exists() {
        std::fs::create_dir(&thumbnails_dir).expect("Failed to create thumbnails_dir");
    }
    if thumbnails_dir.exists() && thumbnails_dir.is_file() {
        error!("thumbnails_dir is not a directory: {:?}", thumbnails_dir);
        std::process::exit(1);
    }

    let app_config = AppConfig {
        tg_homedir,
        db_filepath,
        thumbnails_dir,
    };
    info!("Config: {:?}", app_config);

    app_config
}
