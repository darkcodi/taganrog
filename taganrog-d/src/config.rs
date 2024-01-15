use clap::Parser;

#[derive(Parser, Debug)]
pub struct FlatConfig {
    #[arg(env = "DATABASE_URL", required = true, help = "Postgres connection string")]
    database_url: String,

    #[arg(env = "API_TOKEN", required = true, help = "API Bearer token")]
    bearer_token: String,
}

#[derive(Debug)]
pub struct Config {
    pub db: DbConfiguration,
    pub api: ApiConfiguration,
}

#[derive(Debug)]
pub struct DbConfiguration {
    pub database_url: String, // DATABASE_URL
}

#[derive(Debug)]
pub struct ApiConfiguration {
    pub bearer_token: String, // API_TOKEN
}

impl From<FlatConfig> for Config {
    fn from(value: FlatConfig) -> Self {
        Config {
            db: DbConfiguration {
                database_url: value.database_url,
            },
            api: ApiConfiguration {
                bearer_token: value.bearer_token,
            },
        }
    }
}
