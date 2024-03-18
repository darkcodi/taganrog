use clap::{Arg, Command};
use log::{debug, error, info};
use taganrog::{cli, config, web_ui};
use taganrog::client::TaganrogClient;
use taganrog::config::AppConfig;
use taganrog::entities::{InsertResult};

#[tokio::main]
async fn main() {
    let app = Command::new("tgk")
        .version("0.1")
        .author("Ivan Yaremenchuk")
        .about("Taganrog All-In-One binary: CLI, Web UI")
        .arg(Arg::new("config-path")
            .required(false)
            .help("Set the config file path. Configuration file is optional. Default: $HOME/taganrog.config.toml")
            .long("config-path")
            .short('c')
            .global(true)
            .env("TAG_CONFIG"))
        .arg(Arg::new("work-dir")
            .required(false)
            .help("Override working directory, where the database is stored. Only files in this directory and its subdirectories can be tagged. Default: $HOME")
            .long("workdir")
            .short('w')
            .global(true)
            .env("TAG_WORK_DIR"))
        .arg(Arg::new("upload-dir")
            .required(false)
            .help("Override media upload directory, which is used only by Web UI. It should be a subdirectory of the working directory. Default: $WORKDIR/taganrog-uploads")
            .long("upload-dir")
            .short('u')
            .global(true)
            .env("TAG_UPLOAD_DIR"))
        .arg(Arg::new("verbose")
            .required(false)
            .num_args(0)
            .help("Print debug information")
            .long("verbose")
            .short('v')
            .global(true)
            .env("TAG_VERBOSE"))
        .subcommand_required(true)
        .subcommand(
            Command::new("config")
                .about("Manage file configuration")
                .subcommand(
                    Command::new("get")
                        .about("Get a configuration value")
                        .arg(Arg::new("key").required(true).help("Key of the configuration value"))
                )
                .subcommand(
                    Command::new("set")
                        .about("Set a configuration value")
                        .arg(Arg::new("key").required(true).help("Key of the configuration value"))
                        .arg(Arg::new("value").required(true).help("Value of the configuration value"))
                )
        )
        .subcommand(
            Command::new("web-ui")
                .about("Serve a web-ui using the Axum framework")
        )
        .subcommand(
            Command::new("add")
                .about("Add a file to the database")
                .arg(Arg::new("filepath").required(true).help("File(s) to add").num_args(1..).value_delimiter(' ')),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove a file from the database")
                .arg(Arg::new("filepath").required(true).help("File(s) to remove").num_args(1..).value_delimiter(' ')),
        )
        .subcommand(
            Command::new("tag")
                .about("Tag a file. It also adds the file to the database if it's not there yet.")
                .arg(Arg::new("filepath").required(true).help("Path of the file to tag"))
                .arg(Arg::new("tag").required(true).help("Tag(s) to add").num_args(1..).value_delimiter(',')),
        )
        .subcommand(
            Command::new("untag")
                .about("Untag a file")
                .arg(Arg::new("filepath").required(true).help("Path of the file to untag"))
                .arg(Arg::new("tag").required(true).help("Tag(s) to remove").num_args(1..).value_delimiter(',')),
        );

    let matches = app.get_matches();

    match matches.subcommand() {
        Some(("config", config_matches)) => {
            config::configure_console_logging(&matches);
            let config = config::get_app_config(&matches);
            match config_matches.subcommand() {
                Some(("get", get_matches)) => {
                    let key: &String = get_matches.get_one("key").unwrap();
                    cli::get_config_value(config, key)
                },
                Some(("set", set_matches)) => {
                    let key: &String = set_matches.get_one("key").unwrap();
                    let value: &String = set_matches.get_one("value").unwrap();
                    cli::set_config_value(config, key, value)
                },
                _ => {
                    error!("Invalid subcommand");
                    std::process::exit(1);
                }
            }
        },
        Some(("web-ui", _)) => {
            config::configure_api_logging(&matches);
            let config = config::get_app_config(&matches);
            web_ui::serve(config).await
        },
        Some(("add", add_matches)) => {
            config::configure_console_logging(&matches);
            let config = config::get_app_config(&matches);
            let filepath_vec: Vec<&String> = add_matches.get_many("filepath").unwrap().collect();
            let mut client = create_taganrog_client(config).await;
            for filepath in filepath_vec {
                match cli::add_media(&mut client, filepath).await {
                    Ok(insert_result) => {
                        match insert_result {
                            InsertResult::Existing(existing_media) => { info!("Media already exists: {:?}", existing_media); }
                            InsertResult::New(new_media) => { info!("Added media: {:?}", new_media); }
                        }
                    },
                    Err(e) => {
                        error!("Failed to add media: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },
        Some(("remove", remove_matches)) => {
            config::configure_console_logging(&matches);
            let config = config::get_app_config(&matches);
            let filepath_vec: Vec<&String> = remove_matches.get_many("filepath").unwrap().collect();
            let mut client = create_taganrog_client(config).await;
            for filepath in filepath_vec {
                match cli::remove_media(&mut client, filepath).await {
                    Ok(maybe_media) => {
                        match maybe_media {
                            Some(media) => { info!("Removed media: {:?}", media); }
                            None => { info!("Media not found"); }
                        }
                    },
                    Err(e) => {
                        error!("Failed to remove media: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },
        Some(("tag", _)) => {
            // let filepath: &String = tag_matches.get_one("filepath").unwrap();
            // let tags: Vec<&String> = tag_matches.get_many("tag").unwrap().collect();
        },
        Some(("untag", _)) => {
        },
        _ => {
            error!("Invalid subcommand");
            std::process::exit(1);
        }
    }
}

async fn create_taganrog_client(config: AppConfig) -> TaganrogClient {
    let mut client = TaganrogClient::new(config.clone());
    let init_result = client.init().await;
    if init_result.is_err() {
        error!("Failed to initialize client: {}", init_result.err().unwrap());
        std::process::exit(1);
    }
    client
}
