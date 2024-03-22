use clap::{Arg, Command};
use log::{debug, error, info};
use taganrog::{cli, config, web_ui};
use taganrog::client::TaganrogClient;
use taganrog::config::AppConfig;
use taganrog::entities::{InsertResult};
use taganrog::storage::FileStorage;

#[tokio::main]
async fn main() {
    let command = Command::new("tgk")
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
                .arg(Arg::new("tag").required(true).help("Tag(s) to add").num_args(1..).value_delimiter(' ')),
        )
        .subcommand(
            Command::new("untag")
                .about("Untag a file")
                .arg(Arg::new("filepath").required(true).help("Path of the file to untag"))
                .arg(Arg::new("tag").required(true).help("Tag(s) to remove").num_args(1..).value_delimiter(' ')),
        )
        .subcommand(
            Command::new("list")
                .about("Search tags")
                .arg(Arg::new("all").required(false).help("List all tags").long("all").short('a').action(clap::ArgAction::SetTrue))
                .arg(Arg::new("tag").required(false).help("Tag name")),
        )
        .subcommand(
            Command::new("search")
                .about("Search media")
                .arg(Arg::new("page").required(false).help("Page number").long("page").short('p').default_value("1"))
                .arg(Arg::new("page-size").required(false).help("Page size").long("page-size").short('s').default_value("10"))
                .arg(Arg::new("all").required(false).help("List all media").long("all").short('a').action(clap::ArgAction::SetTrue))
                .arg(Arg::new("tag").required(true).help("List of tags that is used for AND-matching media").num_args(1..).value_delimiter(' ')),
        );

    handle_command(command).await;
}

async fn handle_command(command: Command) {
    let matches = command.get_matches();
    match matches.subcommand() {
        Some(("config", config_matches)) => {
            config::configure_console_logging(&matches);
            match config_matches.subcommand() {
                Some(("get", get_matches)) => {
                    let key: &String = get_matches.get_one("key").unwrap();
                    let config_path = config::get_config_path(&matches)
                        .expect("Failed to get config path");
                    cli::get_config_value(&config_path, key)
                },
                Some(("set", set_matches)) => {
                    let key: &String = set_matches.get_one("key").unwrap();
                    let value: &String = set_matches.get_one("value").unwrap();
                    let config_path = config::get_config_path(&matches)
                        .expect("Failed to get config path");
                    cli::set_config_value(&config_path, key, value)
                },
                _ => {
                    error!("Invalid subcommand");
                    std::process::exit(1);
                }
            }
        },
        Some(("web-ui", _)) => {
            config::configure_api_logging(&matches);
            let config = config::get_app_config_or_exit(&matches);
            let client = create_taganrog_client(config).await;
            web_ui::serve(client).await
        },
        Some(("add", add_matches)) => {
            config::configure_console_logging(&matches);
            let config = config::get_app_config_or_exit(&matches);
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
            let config = config::get_app_config_or_exit(&matches);
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
        Some(("tag", tag_matches)) => {
            config::configure_console_logging(&matches);
            let filepath: &String = tag_matches.get_one("filepath").unwrap();
            let tags: Vec<&String> = tag_matches.get_many("tag").unwrap().collect();
            let config = config::get_app_config_or_exit(&matches);
            let mut client = create_taganrog_client(config).await;
            for tag in tags {
                match cli::tag_media(&mut client, filepath, tag).await {
                    Ok(was_added) => {
                        if was_added {
                            info!("Tagged media: {}", filepath);
                        } else {
                            info!("Media already has tag: {}", filepath);
                        }
                    },
                    Err(e) => {
                        error!("Failed to tag media: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },
        Some(("untag", untag_matches)) => {
            config::configure_console_logging(&matches);
            let filepath: &String = untag_matches.get_one("filepath").unwrap();
            let tags: Vec<&String> = untag_matches.get_many("tag").unwrap().collect();
            let config = config::get_app_config_or_exit(&matches);
            let mut client = create_taganrog_client(config).await;
            for tag in tags {
                match cli::untag_media(&mut client, filepath, tag).await {
                    Ok(was_removed) => {
                        if was_removed {
                            info!("Untagged media: {}", filepath);
                        } else {
                            info!("Media does not have tag: {}", filepath);
                        }
                    },
                    Err(e) => {
                        error!("Failed to untag media: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },
        Some(("list", list_matches)) => {
            config::configure_console_logging(&matches);
            let all: bool = list_matches.get_flag("all");
            let max_items = if all { usize::MAX } else { 10 };
            let tag_name: String = list_matches.get_one::<String>("tag").map(|x| x.to_owned()).unwrap_or_default();
            let config = config::get_app_config_or_exit(&matches);
            let client = create_taganrog_client(config).await;
            let tags_autocomplete = cli::list_tags(&client, tag_name, max_items).await;
            for tag_autocomplete in tags_autocomplete {
                info!("[{}] {}", tag_autocomplete.media_count, tag_autocomplete.last);
            }
        },
        Some(("search", search_matches)) => {
            config::configure_console_logging(&matches);
            let mut page: usize = search_matches.get_one::<String>("page").and_then(|x| x.parse::<usize>().ok()).unwrap_or(1);
            let mut page_size: usize = search_matches.get_one::<String>("page-size").and_then(|x| x.parse::<usize>().ok()).unwrap_or(10);
            let all: bool = search_matches.get_flag("all");
            if all { page_size = usize::MAX; page = 1; }
            let tags: Vec<String> = search_matches.get_many::<String>("tag").unwrap().map(|x| x.to_owned()).collect();
            let config = config::get_app_config_or_exit(&matches);
            let client = create_taganrog_client(config).await;
            let page_index = page - 1;
            let media_page = cli::search_media(&client, tags, page_size, page_index).await;

            info!("Displaying page {}/{}", media_page.page_index + 1, media_page.total_pages);
            info!("Total results: {}", media_page.total_count);
            for media in media_page.media_vec {
                info!("{}: {}", media.location, media.tags.join(", "));
            }
        },
        _ => {
            error!("Invalid subcommand");
            std::process::exit(1);
        }
    }
}

async fn create_taganrog_client(config: AppConfig) -> TaganrogClient<FileStorage> {
    debug!("Initializing storage...");
    let storage_result = FileStorage::new(config.work_dir.clone());
    if storage_result.is_err() {
        error!("Failed to initialize storage: {}", storage_result.err().unwrap());
        std::process::exit(1);
    }
    let storage = storage_result.unwrap();
    debug!("Storage initialized!");

    let mut client = TaganrogClient::new(config.clone(), storage);

    debug!("Initializing DB...");
    let init_result = client.init().await;
    if init_result.is_err() {
        error!("Failed to initialize client: {}", init_result.err().unwrap());
        std::process::exit(1);
    }
    debug!("DB Initialized!");

    client
}
