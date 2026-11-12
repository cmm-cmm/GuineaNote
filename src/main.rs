use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "guinea",
    version,
    about = "Unicode-aware terminal text editor",
    after_help = "Open files with: guinea a.txt b.txt\nMissing paths become new buffers; Ctrl+O opens the file picker."
)]
struct Args {
    /// Files to open. Missing file paths become unsaved buffers; folders are rejected.
    files: Vec<PathBuf>,
    /// Disable document edits and saves.
    #[arg(long)]
    readonly: bool,
    /// Read editor configuration from a TOML file.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
    /// Interface language: Vietnamese or English (F10 opens Settings).
    #[arg(long, value_parser = ["vi", "en"])]
    language: Option<String>,
    /// Enable local Agent access to this workspace and unsaved buffers.
    #[arg(long)]
    agent: bool,
    /// Grant Agent patch/save/new/close capabilities for this session.
    #[arg(long, requires = "agent", conflicts_with = "readonly")]
    agent_write: bool,
    /// Explicitly allow Agent reads of sensitive paths such as .env.
    #[arg(long, requires = "agent")]
    agent_secrets: bool,
}

fn main() -> std::process::ExitCode {
    let args = Args::parse();
    match guinea_tui::run_with_agent(
        args.files,
        args.readonly,
        args.config,
        args.language.as_deref(),
        guinea_tui::AgentOptions {
            enabled: args.agent,
            write: args.agent_write,
            secrets: args.agent_secrets,
        },
    ) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Guinea Note: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
