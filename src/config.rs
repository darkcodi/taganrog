use std::path::{Path, PathBuf};
use clap::ArgMatches;
use home::home_dir;
use log::{debug, error, LevelFilter};
use path_absolutize::Absolutize;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::utils::str_utils::StringExtensions;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ConfigBuilder {
    pub workdir: Option<String>,
    pub upload_dir: Option<String>,
}

impl ConfigBuilder {
    pub fn merge(&self, other: &ConfigBuilder) -> ConfigBuilder {
        ConfigBuilder {
            workdir: self.workdir.clone().or(other.workdir.clone()),
            upload_dir: self.upload_dir.clone().or(other.upload_dir.clone()),
        }
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to get home directory")]
    HomeDirNotFound,
    #[error("Failed to get current directory")]
    CurrentDirNotFound,
    #[error("Failed to read/write config file: {0}")]
    ConfigFileIO(#[from] std::io::Error),
    #[error("Failed to serialize/deserialize config file: {0}")]
    ConfigFileSerialization(#[from] serde_json::Error),
    #[error("Failed to canonicalize path: {0}")]
    PathCanonicalization(std::io::Error),
    #[error("Validation error: {0}")]
    Validation(String),
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub work_dir: PathBuf,
    pub upload_dir: PathBuf,
    pub thumbnails_dir: PathBuf,
}

impl AppConfig {
    pub fn new(builder: ConfigBuilder) -> Result<Self, ConfigError> {
        let work_dir = builder.workdir
            .ok_or_else(|| ConfigError::Validation("workdir is not set".to_string()))?;
        let upload_dir = builder.upload_dir
            .ok_or_else(|| ConfigError::Validation("upload_dir is not set".to_string()))?;

        let work_dir = PathBuf::from(work_dir);
        if !work_dir.exists() {
            std::fs::create_dir_all(&work_dir)?;
        }
        if !work_dir.is_dir() {
            return Err(ConfigError::Validation("workdir is not a directory".to_string()));
        }

        let upload_dir = PathBuf::from(upload_dir);
        if !upload_dir.starts_with(&work_dir) {
            return Err(ConfigError::Validation("upload_dir is not a subdirectory of workdir".to_string()));
        }
        if !upload_dir.exists() {
            std::fs::create_dir_all(&upload_dir)?;
        }
        if upload_dir.exists() && !upload_dir.is_dir() {
            return Err(ConfigError::Validation("upload_dir is not a directory".to_string()));
        }
        let thumbnails_dir = work_dir.join("taganrog-thumbnails");
        if !thumbnails_dir.exists() {
            std::fs::create_dir_all(&thumbnails_dir)?;
        }
        if thumbnails_dir.exists() && !thumbnails_dir.is_dir() {
            return Err(ConfigError::Validation("thumbnails_dir is not a directory".to_string()));
        }

        Ok(Self { work_dir, upload_dir, thumbnails_dir })
    }
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

    fern::Dispatch::new()
        // Perform allocation-free log formatting
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339(std::time::SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(min_level)
        .level_for("tower_http::trace::on_response", LevelFilter::Debug)
        .level_for("tower_http::trace::make_span", LevelFilter::Debug)
        .chain(std::io::stdout())
        .apply()
        .expect("Failed to configure logging");
}

pub fn get_app_config_or_exit(matches: &ArgMatches) -> AppConfig {
    let config_path_result = get_config_path(matches);
    if let Err(e) = &config_path_result {
        error!("Failed to get config path: {}", e);
        std::process::exit(1);
    }
    let config_path = config_path_result.unwrap();
    let appconfig_result = get_app_config(matches, &config_path);
    if let Err(e) = &appconfig_result {
        error!("Failed to get app config: {}", e);
        std::process::exit(1);
    }
    let app_config = appconfig_result.unwrap();
    debug!("{:?}", &app_config);

    app_config
}

pub fn get_app_config(matches: &ArgMatches, config_path: &Path) -> Result<AppConfig, ConfigError> {
    let home_dir = get_home_dir()?;
    let env_config = read_env_config(matches)?;
    let file_config = read_file_config(config_path)?;

    let mut final_config = env_config.merge(&file_config);
    if final_config.workdir.is_none() {
        let default_workdir = home_dir.display().to_string();
        final_config.workdir = Some(default_workdir);
    }
    if final_config.upload_dir.is_none() {
        let workdir = final_config.workdir.as_ref().unwrap();
        let default_upload_dir = Path::new(workdir).join("taganrog-uploads").display().to_string();
        final_config.upload_dir = Some(default_upload_dir);
    }
    debug!("Final config: {:?}", &final_config);

    let app_config = AppConfig::new(final_config)?;
    Ok(app_config)
}

pub fn get_home_dir() -> Result<PathBuf, ConfigError> {
    let maybe_homedir_path = home_dir();
    if maybe_homedir_path.is_none() {
        return Err(ConfigError::HomeDirNotFound);
    }
    let homedir_path = maybe_homedir_path.unwrap();
    if homedir_path.as_os_str().is_empty() {
        return Err(ConfigError::HomeDirNotFound);
    }
    Ok(homedir_path)
}

pub fn get_config_path(matches: &ArgMatches) -> Result<PathBuf, ConfigError> {
    let home_dir = get_home_dir()?;
    let maybe_config_path: Option<String> = matches.get_one("config-path").and_then(|x: &String| x.empty_to_none());
    if let Some(config_path) = &maybe_config_path {
        let config_path = PathBuf::from(config_path);
        debug!("TAG_CONFIG: {}", config_path.display().to_string());
        Ok(config_path)
    } else {
        let config_path = home_dir.join("taganrog.config.json");
        debug!("No custom config path set. Using default: {}", config_path.display().to_string());
        Ok(config_path)
    }
}

pub fn read_file_config(config_path: &Path) -> Result<ConfigBuilder, ConfigError> {
    if !config_path.exists() {
        debug!("Config file not found: {}", config_path.display());
        return Ok(ConfigBuilder::default());
    }

    let file_content = std::fs::read_to_string(config_path)?;
    let mut config: ConfigBuilder = serde_json::from_str(&file_content)?;
    let work_dir = config.workdir.as_ref().and_then(|x: &String| x.empty_to_none());
    if let Some(work_dir) = work_dir {
        config.workdir = Some(absolute_from(&work_dir, config_path)?);
    } else {
        config.workdir = None;
    }

    let upload_dir = config.upload_dir.as_ref().and_then(|x: &String| x.empty_to_none());
    if let Some(upload_dir) = upload_dir {
        config.upload_dir = Some(absolute_from(&upload_dir, config_path)?);
    } else {
        config.upload_dir = None;
    }

    debug!("File config: {:?}", &config);
    Ok(config)
}

pub fn write_file_config(config_path: &Path, config: &ConfigBuilder) -> Result<(), ConfigError> {
    let config_json = serde_json::to_string_pretty(config)?;
    std::fs::write(config_path, config_json)?;
    Ok(())
}

pub fn read_env_config(matches: &ArgMatches) -> Result<ConfigBuilder, ConfigError> {
    let current_dir = std::env::current_dir().map_err(|_| ConfigError::CurrentDirNotFound)?;

    let mut config = ConfigBuilder::default();
    let work_dir = matches.get_one("work-dir").and_then(|x: &String| x.empty_to_none());
    if let Some(work_dir) = work_dir {
        config.workdir = Some(absolute_from(&work_dir, &current_dir)?);
    }

    let upload_dir = matches.get_one("upload-dir").and_then(|x: &String| x.empty_to_none());
    if let Some(upload_dir) = upload_dir {
        config.upload_dir = Some(absolute_from(&upload_dir, &current_dir)?);
    }

    debug!("Env config: {:?}", &config);
    Ok(config)
}

fn absolute_from(path: &str, base: &Path) -> Result<String, ConfigError> {
    let canonical_path = Path::new(path).absolutize_from(base).and_then(|x| x.canonicalize())
        .map(|x| x.display().to_string())
        .map_err(ConfigError::PathCanonicalization)?;
    Ok(canonical_path)
}
