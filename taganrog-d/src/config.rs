use clap::Parser;

#[derive(Parser, Debug)]
pub struct FlatConfig {
    #[arg(env = "DATABASE_URL", required = true, help = "Postgres connection string")]
    database_url: String,
}

#[derive(Debug)]
pub struct Config {
    pub db: DbConfiguration,
}

#[derive(Debug)]
pub struct DbConfiguration {
    pub database_url: String, // DATABASE_URL
}

impl From<FlatConfig> for Config {
    fn from(value: FlatConfig) -> Self {
        Config {
            db: DbConfiguration {
                database_url: value.database_url,
            },
        }
    }
}
