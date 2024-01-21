use clap::Parser;
use path_absolutize::Absolutize;

#[derive(Parser, Debug)]
pub struct PlainConfig {
    #[arg(env = "TAG_WORKDIR", default_value = ".", help = "Working directory for the taganrog-d server")]
    pub workdir: String,
}

#[derive(Debug)]
pub struct Config {
    pub workdir: std::path::PathBuf,
    pub db_path: std::path::PathBuf,
}

impl Config {
    pub fn parse() -> anyhow::Result<Self> {
        let config: PlainConfig = PlainConfig::parse();
        let workdir = std::path::Path::new(&config.workdir).absolutize_from(std::env::current_dir()?)?;
        if !workdir.exists() {
            std::fs::create_dir_all(&workdir)?;
        }
        if !workdir.is_dir() {
            anyhow::bail!("workdir is not a directory");
        }
        let workdir = workdir.canonicalize()?;
        let db_path = workdir.join("taganrog.db");
        if db_path.exists() && !db_path.is_file() {
            anyhow::bail!("db_path is not a file");
        }
        Ok(Config {
            workdir,
            db_path,
        })
    }
}
