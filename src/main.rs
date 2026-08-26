use std::path::PathBuf;

use gitbolt::ui::{parse_cli_path, App, CliLaunchPath};

fn main() {
    let path: Option<PathBuf> = parse_cli_path(std::env::args_os());
    dioxus::LaunchBuilder::desktop()
        .with_context(CliLaunchPath(path))
        .launch(App);
}
