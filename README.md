<p align="center">
  <h1 align="center">🔖TAGanrog</h1>

  <p align="center">
    A personal <b>tagging system</b> and a <b>search engine</b> for your media library.
    <br/>
  </p>
</p>

## Table Of Contents

* [About the Project](#about-the-project)
* [Demo](#demo)
* [Features](#features)
* [Built With](#built-with)
* [Installation](#installation)
* [Usage](#usage)
  * [CLI](#cli)
  * [Desktop](#desktop)
* [License](#license)

## About the Project

In the digital age, where the quantity of files and data we handle is enormous, finding the exact file you need can be like looking for a needle in a haystack. Taganrog is designed to solve this problem by allowing you to tag your files with custom tags and then search through them as easily as you would search the web using Google. Whether it's documents, images, videos, or any other file type, Taganrog brings order to chaos, making your digital life more organized and efficient.

## Demo

![DemoRecording](demo.gif)

## Features

- ✨ **Google-like UI**: Search your files in a neat, Google-like search bar.
- ⚡ **Blazingly Fast**: tags autocompletion, searching media files, adding/removing tags, everything works within milliseconds(!)
- 💾 **Local Storage**: All your tags and files are stored locally on your machine. There is NO server.
- 🖥️ **CLI**: Taganrog is also a CLI tool that allows you to manage your tags and files from the command line.
- 📦 **Portable**: Taganrog is a single binary (that includes both - the CLI and the desktop app), that you can run on any platform without any dependencies.
- 📤 **Exportable**: The entire DB is just a single JSON file that is human-readable and can be easily exported to other systems.

## Built With

This project was built using the following open-source frameworks/libraries:
- for desktop app: [Tauri](https://github.com/tauri-apps/tauri)
- for UI templating: [Axum](https://github.com/tokio-rs/axum) + [Askama](https://github.com/djc/askama)
- for CLI: [Clap](https://github.com/clap-rs/clap)
- for DB: append-only JSON file (using [serde](https://github.com/serde-rs/serde))

## Installation

There are four ways to install Taganrog:

1. **Using Cargo**:
   - If you have Rust installed, you can install Taganrog using Cargo:
    ```sh
    cargo install taganrog
    ```

2. **Building from source**:
   - Clone the repo and build the project using Cargo:
    ```sh
    git clone https://github.com/darkcodi/taganrog.git
    cd taganrog
    cargo build --release
    ```
   - The binary will be available at `target/release/taganrog`
   - [Linux only] You can also install the binary to your system using:
    ```sh
    sudo cp target/release/taganrog /usr/local/bin
    ```
   - [Windows only] You can also install the binary to your system by adding the `target\release` directory to your PATH.

3. **[COMING SOON] Using the pre-built binaries**:
   - Download the latest binary from the [releases page](https://github.com/darkcodi/taganrog/releases) and run it.

## Usage

### CLI

Taganrog can be used as a CLI tool to manage your tags and files. Here are some of the available commands:
- `taganrog tag <file> <tag1> [tag2 ...]`: Tag a file with one or more tags.
- `taganrog untag <file> <tag1> [tag2 ...]`: Remove one or more tags from a file.
- `taganrog list [tag]`: List all tags that start with a specific prefix. If no prefix is provided, all tags are listed.
- `taganrog search <tag1> [tag2 ...]`: Search for files with a specific tag or tags.

### Desktop

If you launch Taganrog without any arguments, it will start a desktop app that you can use to manage your tags and files. Here are some of the available features:
- **Search**: Enter tags in the search bar to search for files that have those tags.
- **Tag new files**: Click on the `Plus` button in the top right corner and select a file(s) to tag.
- **Add/Delete Tags**: Click on some media file and then add/remove tags to it on the right-side panel.
- **Delete Files**: Open a media by clicking it and press the `Delete` button on the right-side pane to delete it.
- **Tags Cloud**: Click on the `Cloud` button in the top right corner to see a cloud of your top 100 used tags.

## License

Distributed under the MIT License. See [LICENSE](https://github.com/darkcodi/taganrog/blob/main/LICENSE) for more information.
