use std::path::PathBuf;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct FlatConfig {
    #[arg(env = "TAG_WORKDIR", default_value = ".", help = "Working directory for the taganrog-d server")]
    pub workdir: String,
}

#[derive(Debug)]
pub struct Config {
    pub workdir: PathBuf,
    pub db_path: PathBuf,
}

impl Config {
    pub fn parse() -> anyhow::Result<Self> {
        let flat_config = FlatConfig::parse();
        let workdir = std::env::current_dir()?
           .join(flat_config.workdir).canonicalize()?;
        let db_path = workdir.join("taganrog.db");
        Ok(Self {
            workdir,
            db_path,
        })
    }
}
