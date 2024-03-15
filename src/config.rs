use std::path::{Path, PathBuf};
use clap::ArgMatches;
use home::home_dir;
use path_absolutize::Absolutize;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, Level};
use tracing_subscriber::filter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use crate::utils::str_utils::StringExtensions;

#[derive(Debug, Default, Serialize, Deserialize)]
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

#[derive(Debug)]
pub struct AppConfig {
    pub config_path: PathBuf,
    pub file_config: ConfigBuilder,

    // final config
    pub workdir: PathBuf,
    pub upload_dir: PathBuf,
    pub db_path: PathBuf,
    pub thumbnails_dir: PathBuf,
}

impl AppConfig {
    pub fn new(config_path: PathBuf, file_config: ConfigBuilder, final_config: ConfigBuilder) -> anyhow::Result<Self> {
        let workdir = final_config.workdir.ok_or_else(|| anyhow::anyhow!("workdir is not set"))?;
        let upload_dir = final_config.upload_dir.ok_or_else(|| anyhow::anyhow!("upload_dir is not set"))?;

        let workdir = Self::get_or_create_workdir(&workdir)?;
        let upload_dir = Self::get_or_create_upload_dir(&workdir, &upload_dir)?;
        let db_path = Self::get_or_create_db_path(&workdir)?;
        let thumbnails_dir = Self::get_or_create_thumbnails_dir(&workdir)?;
        Ok(Self { config_path, file_config, workdir, upload_dir, db_path, thumbnails_dir })
    }

    fn get_or_create_workdir(workdir: &str) -> anyhow::Result<PathBuf> {
        let workdir = PathBuf::from(workdir);
        if !workdir.exists() {
            std::fs::create_dir_all(&workdir)?;
        }
        if !workdir.is_dir() {
            anyhow::bail!("workdir is not a directory");
        }
        Ok(workdir)
    }

    fn get_or_create_upload_dir(workdir: &PathBuf, upload_dir: &str) -> anyhow::Result<PathBuf> {
        let upload_dir = PathBuf::from(upload_dir);
        if !upload_dir.starts_with(workdir) {
            anyhow::bail!("upload_dir is not a subdirectory of workdir");
        }
        if !upload_dir.exists() {
            std::fs::create_dir_all(&upload_dir)?;
        }
        if upload_dir.exists() && !upload_dir.is_dir() {
            anyhow::bail!("upload_dir is not a directory");
        }
        Ok(upload_dir)
    }

    fn get_or_create_db_path(workdir: &Path) -> anyhow::Result<PathBuf> {
        let db_path = workdir.join("taganrog.db.json");
        if !db_path.exists() {
            std::fs::write(&db_path, "")?;
        }
        if db_path.exists() && !db_path.is_file() {
            anyhow::bail!("db_path is not a file");
        }
        Ok(db_path)
    }

    fn get_or_create_thumbnails_dir(workdir: &Path) -> anyhow::Result<PathBuf> {
        let thumbnails_dir = workdir.join("taganrog-thumbnails");
        if !thumbnails_dir.exists() {
            std::fs::create_dir_all(&thumbnails_dir)?;
        }
        if thumbnails_dir.exists() && !thumbnails_dir.is_dir() {
            anyhow::bail!("thumbnails_dir is not a directory");
        }
        Ok(thumbnails_dir)
    }
}

pub fn configure_logging(matches: &ArgMatches) {
    let is_verbose = matches.get_one("verbose").map(|x: &bool| x.to_owned()).unwrap_or_default();

    let tracing_layer = tracing_subscriber::fmt::layer();
    let default_level = if is_verbose { Level::DEBUG } else { Level::INFO };
    let mut filter = filter::Targets::new().with_default(default_level);
    if is_verbose {
        filter = filter
            // .with_target("tower_http::trace::on_request", Level::DEBUG)
            .with_target("tower_http::trace::on_response", Level::DEBUG)
            .with_target("tower_http::trace::make_span", Level::DEBUG);
    }
    tracing_subscriber::registry()
        .with(tracing_layer)
        .with(filter)
        .init();
}

pub fn get_app_config(matches: &ArgMatches) -> AppConfig {
    let home_dir = get_home_dir();
    let config_path = get_config_path(&home_dir, matches);
    let env_config = read_env_config(matches);
    let file_config = read_file_config(&config_path).unwrap_or_default();

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

    AppConfig::new(config_path, file_config, final_config).expect("Failed to create AppConfig")
}

fn get_home_dir() -> PathBuf {
    let maybe_homedir_path = home_dir();
    if maybe_homedir_path.is_none() {
        error!("Failed to get home directory: homedir is None");
        std::process::exit(1);
    }
    let homedir_path = maybe_homedir_path.unwrap();
    if homedir_path.as_os_str().is_empty() {
        error!("Failed to get home directory: homedir is empty");
        std::process::exit(1);
    }
    homedir_path
}

fn get_config_path(home_dir: &Path, matches: &ArgMatches) -> PathBuf {
    let maybe_config_path: Option<String> = matches.get_one("config-path").and_then(|x: &String| x.empty_to_none());
    if let Some(config_path) = &maybe_config_path {
        let path_buf_result = PathBuf::try_from(config_path);
        if path_buf_result.is_err() {
            error!("Failed to convert config path to PathBuf: {}", path_buf_result.err().unwrap());
            std::process::exit(1);
        }
        let config_path = path_buf_result.unwrap();
        debug!("TAG_CONFIG: {}", config_path.display().to_string());
        config_path
    } else {
        let config_path = home_dir.join("taganrog.config.json");
        debug!("No custom config path set. Using default: {}", config_path.display().to_string());
        config_path
    }
}

pub fn read_file_config(config_path: &Path) -> Option<ConfigBuilder> {
    if !config_path.exists() {
        debug!("Config file not found: {}", config_path.display());
        return None;
    }

    let file_content_result = std::fs::read_to_string(config_path);
    if file_content_result.is_err() {
        error!("Failed to read config file: {}", file_content_result.err().unwrap());
        std::process::exit(1);
    }

    let file_content = file_content_result.unwrap();
    let config_result = serde_json::from_str(&file_content);
    if config_result.is_err() {
        error!("Failed to deserialize config file: {}", config_result.err().unwrap());
        std::process::exit(1);
    }

    let mut config: ConfigBuilder = config_result.unwrap();

    let work_dir = config.workdir.as_ref().and_then(|x: &String| x.empty_to_none());
    if let Some(work_dir) = work_dir {
        let canonical_workdir_result = Path::new(&work_dir).absolutize_from(config_path).and_then(|x| x.canonicalize());
        if canonical_workdir_result.is_err() {
            error!("Config file has invalid work directory: {}", canonical_workdir_result.err().unwrap());
            std::process::exit(1);
        }
        let canonical_workdir = canonical_workdir_result.unwrap().display().to_string();
        config.workdir = Some(canonical_workdir);
    } else {
        config.workdir = None;
    }

    let upload_dir = config.upload_dir.as_ref().and_then(|x: &String| x.empty_to_none());
    if let Some(upload_dir) = upload_dir {
        let canonical_upload_dir_result = Path::new(&upload_dir).absolutize_from(config_path).and_then(|x| x.canonicalize());
        if canonical_upload_dir_result.is_err() {
            error!("Config file has invalid upload directory: {}", canonical_upload_dir_result.err().unwrap());
            std::process::exit(1);
        }
        let upload_dir = canonical_upload_dir_result.unwrap().display().to_string();
        config.upload_dir = Some(upload_dir);
    } else {
        config.upload_dir = None;
    }

    debug!("File config: {:?}", &config);
    Some(config)
}

pub fn write_file_config(config_path: &Path, config: &ConfigBuilder) {
    let config_json_result = serde_json::to_string_pretty(config);
    if config_json_result.is_err() {
        error!("Failed to serialize config to JSON: {}", config_json_result.err().unwrap());
        std::process::exit(1);
    }

    let config_json = config_json_result.unwrap();
    let write_result = std::fs::write(config_path, config_json);
    if write_result.is_err() {
        error!("Failed to write config to file: {}", write_result.err().unwrap());
        std::process::exit(1);
    }
}

fn read_env_config(matches: &ArgMatches) -> ConfigBuilder {
    let current_dir_result = std::env::current_dir();
    if current_dir_result.is_err() {
        error!("Failed to get current directory: {}", current_dir_result.err().unwrap());
        std::process::exit(1);
    }

    let mut config = ConfigBuilder::default();
    let current_dir = current_dir_result.unwrap();
    let work_dir = matches.get_one("work-dir").and_then(|x: &String| x.empty_to_none());
    if let Some(work_dir) = work_dir {
        let canonical_workdir_result = Path::new(&work_dir).absolutize_from(&current_dir).and_then(|x| x.canonicalize());
        if canonical_workdir_result.is_err() {
            error!("Failed to canonicalize work directory: {}", canonical_workdir_result.err().unwrap());
            std::process::exit(1);
        }
        let canonical_workdir = canonical_workdir_result.unwrap().display().to_string();
        debug!("TAG_WORK_DIR: {}", canonical_workdir);
        config.workdir = Some(canonical_workdir);
    }

    let upload_dir = matches.get_one("upload-dir").and_then(|x: &String| x.empty_to_none());
    if let Some(upload_dir) = upload_dir {
        let canonical_upload_dir_result = Path::new(&upload_dir).absolutize_from(&current_dir).and_then(|x| x.canonicalize());
        if canonical_upload_dir_result.is_err() {
            error!("Failed to canonicalize upload directory: {}", canonical_upload_dir_result.err().unwrap());
            std::process::exit(1);
        }
        let upload_dir = canonical_upload_dir_result.unwrap().display().to_string();
        debug!("TAG_UPLOAD_DIR: {}", upload_dir);
        config.upload_dir = Some(upload_dir);
    }

    debug!("Env config: {:?}", &config);
    config
}
