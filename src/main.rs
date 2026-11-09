use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "guineanote",
    version,
    about = "Unicode-aware terminal text editor",
    after_help = "Open a folder with: guineanote .\nOpen individual files with: guineanote a.txt b.txt\nCtrl+B toggles the explorer; F4 switches panes."
)]
struct Args {
    /// Files or a workspace folder. Missing file paths become unsaved buffers.
    files: Vec<PathBuf>,
    /// Disable document edits and saves.
    #[arg(long)]
    readonly: bool,
    /// Read editor configuration from a TOML file.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
    /// Interface language: Vietnamese or English (F8 opens Settings).
    #[arg(long, value_parser = ["vi", "en"])]
    language: Option<String>,
}

fn main() -> std::process::ExitCode {
    let args = Args::parse();
    match guinea_tui::run(
        args.files,
        args.readonly,
        args.config,
        args.language.as_deref(),
    ) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("GuineaNote: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
