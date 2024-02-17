use clap::{Arg, Command};
use taganrog::{cli, web_ui};
use taganrog::client::TaganrogConfig;

#[tokio::main]
async fn main() {
    let app = Command::new("tgk")
        .version("0.1")
        .author("Ivan Yaremenchuk")
        .about("Taganrog All-In-One binary: CLI, daemon (API), Web UI")
        .arg(Arg::new("workdir")
            .required(false)
            .help("Set the tag working directory (where the database is stored)")
            .long("workdir")
            .short('w')
            .global(true)
            .env("TAG_WORKDIR")
            .default_value("."))
        .arg(Arg::new("upload-dir")
            .required(false)
            .help("Set the media upload directory (should be a subdirectory of the working directory)")
            .long("upload-dir")
            .short('u')
            .global(true)
            .env("UPLOAD_DIR")
            .default_value("."))
        .subcommand_required(true)
        .subcommand(
            Command::new("serve")
                .about("Serve commands (api, web-ui) using the axum framework")
                .subcommand(Command::new("web-ui").about("Serve the web UI"))
        )
        .subcommand(
            Command::new("add")
                .about("Add a file to the database")
                .arg(Arg::new("filepath").required(true).help("Path of the file to add")),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove a file from the database")
                .arg(Arg::new("filepath").required(true).help("Path of the file to remove")),
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
        Some(("serve", serve_matches)) => {
            match serve_matches.subcommand() {
                Some(("web-ui", _)) => {
                    let workdir: &String = matches.get_one("workdir").unwrap();
                    let upload_dir: &String = matches.get_one("upload-dir").unwrap();
                    let config: TaganrogConfig = TaganrogConfig::new(workdir, upload_dir).expect("failed to initialize config");
                    web_ui::serve(config).await
                },
                _ => unreachable!(),
            }
        },
        Some(("add", add_matches)) => {
            let workdir: &String = matches.get_one("workdir").unwrap();
            let upload_dir: &String = matches.get_one("upload-dir").unwrap();
            let config: TaganrogConfig = TaganrogConfig::new(workdir, upload_dir).expect("failed to initialize config");
            let filepath: &String = add_matches.get_one("filepath").unwrap();
            cli::add_media(config, filepath).await
        },
        Some(("remove", _)) => {
        },
        Some(("tag", _)) => {
            // let filepath: &String = tag_matches.get_one("filepath").unwrap();
            // let tags: Vec<&String> = tag_matches.get_many("tag").unwrap().collect();
            // println!("filepath: {}", filepath);
            // println!("tags: {:?}", tags);
        },
        Some(("untag", _)) => {
        },
        _ => {
            eprintln!("Invalid subcommand");
            std::process::exit(1);
        }
    }
}
