use clap::Parser;

#[derive(Parser, Debug)]
pub struct Config {
    #[arg(env = "TAG_WORKDIR", default_value = ".", help = "Working directory for the taganrog-d server")]
    pub workdir: String,
}
