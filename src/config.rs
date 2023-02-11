use clap::Parser;

#[derive(Parser, Debug)]
pub struct FlatConfig {
    #[arg(env = "DATABASE_URL", required = true, help = "Postgres connection string")]
    database_url: String,

    #[arg(env = "S3_ACCOUNT_ID", required = true, help = "S3 account id")]
    s3_account_id: String,

    #[arg(env = "S3_ACCESS_KEY", required = true, help = "S3 access key")]
    s3_access_key: String,

    #[arg(env = "S3_SECRET_KEY", required = true, help = "S3 secret key")]
    s3_secret_key: String,

    #[arg(env = "S3_PUBLIC_URL", required = true, help = "S3 public url (will be prepended to a media name and served as a public url to a media)")]
    s3_public_url_prefix: String,

    #[arg(env = "S3_BUCKET_NAME", required = false, default_value = "media", help = "S3 bucket name for storing media, default = 'media'")]
    s3_bucket_name: String,
}

#[derive(Debug)]
pub struct Config {
    pub db: DbConfiguration,
    pub s3: S3Configuration,
}

#[derive(Debug)]
pub struct DbConfiguration {
    pub database_url: String, // DATABASE_URL
}

#[derive(Debug)]
pub struct S3Configuration {
    pub bucket_name: String, // S3_BUCKET_NAME
    pub account_id: String, // S3_ACCOUNT_ID
    pub access_key: String, // S3_ACCESS_KEY
    pub secret_key: String, // S3_SECRET_KEY
    pub public_url_prefix: String, // S3_PUBLIC_URL
}

impl From<FlatConfig> for Config {
    fn from(value: FlatConfig) -> Self {
        Config {
            db: DbConfiguration {
                database_url: value.database_url,
            },
            s3: S3Configuration {
                bucket_name: value.s3_bucket_name,
                account_id: value.s3_account_id,
                access_key: value.s3_access_key,
                secret_key: value.s3_secret_key,
                public_url_prefix: value.s3_public_url_prefix,
            },
        }
    }
}
