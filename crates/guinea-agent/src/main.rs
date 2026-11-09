use clap::{Args, Parser, Subcommand};
use guinea_agent::{ConnectArgs, Connection};
use guinea_service::{Error, MAX_MESSAGE, Operation, PAGE_BYTES, PAGE_ITEMS, PatchEdit, Request};
use std::{
    io::{self, Read},
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};
#[derive(Parser)]
#[command(
    name = "guineanote-agent",
    version,
    about = "Structured local GuineaNote commands; stdout contains JSON only"
)]
struct Cli {
    #[command(flatten)]
    connect: ConnectArgs,
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true)]
    request_id: Option<String>,
    #[command(subcommand)]
    command: Command,
}
#[derive(Args)]
struct Page {
    #[arg(long, default_value_t = 0)]
    offset: usize,
    #[arg(long,default_value_t=PAGE_ITEMS)]
    limit: usize,
}
#[derive(Args)]
struct Target {
    #[arg(long, conflicts_with = "path")]
    document: Option<String>,
    #[arg(long, conflicts_with = "document")]
    path: Option<String>,
    #[arg(long)]
    expected_hash: Option<String>,
}
#[derive(Subcommand)]
enum Command {
    Capabilities,
    Files {
        #[command(subcommand)]
        command: Files,
    },
    Documents {
        #[command(subcommand)]
        command: Documents,
    },
    Tabs {
        #[command(subcommand)]
        command: Tabs,
    },
    Sessions {
        #[command(subcommand)]
        command: Sessions,
    },
    Patch {
        #[command(subcommand)]
        command: Patch,
    },
    Review {
        #[arg(long)]
        document: String,
        #[arg(long)]
        split: bool,
    },
    Transaction {
        request_id: String,
    },
    /// Read one typed JSON request from a file or '-' (stdin).
    Request {
        #[arg(long)]
        file: PathBuf,
    },
}
#[derive(Subcommand)]
enum Files {
    List {
        #[arg(default_value = "")]
        path: String,
        #[command(flatten)]
        page: Page,
    },
    Info {
        path: String,
    },
}
#[derive(Subcommand)]
enum Documents {
    Read {
        #[command(flatten)]
        target: Target,
        #[arg(long, default_value_t = 0)]
        start: usize,
        #[arg(long,default_value_t=PAGE_BYTES)]
        limit: usize,
    },
    Search {
        needle: String,
        #[command(flatten)]
        target: Target,
        #[arg(long, default_value_t = 0)]
        start: usize,
        #[arg(long,default_value_t=PAGE_ITEMS)]
        limit: usize,
    },
    Analyze {
        #[command(flatten)]
        target: Target,
        #[command(flatten)]
        page: Page,
    },
    Open {
        path: String,
    },
    New,
    Save {
        #[arg(long)]
        document: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        path: Option<String>,
    },
}
#[derive(Subcommand)]
enum Tabs {
    List {
        #[command(flatten)]
        page: Page,
    },
    Activate {
        tab: String,
    },
    Close {
        tab: String,
    },
}
#[derive(Subcommand)]
enum Sessions {
    List,
}
#[derive(Subcommand)]
enum Patch {
    Plan {
        #[arg(long)]
        document: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        edits_file: PathBuf,
    },
    Apply {
        plan: String,
    },
}
fn read_json(path: &PathBuf) -> guinea_service::Result<Vec<u8>> {
    let mut data = Vec::new();
    if path.as_os_str() == "-" {
        io::stdin()
            .take(MAX_MESSAGE as u64 + 1)
            .read_to_end(&mut data)?;
    } else {
        std::fs::File::open(path)?
            .take(MAX_MESSAGE as u64 + 1)
            .read_to_end(&mut data)?;
    }
    if data.len() > MAX_MESSAGE {
        return Err(Error::new("LIMIT_EXCEEDED", "Request file exceeds 2 MiB"));
    }
    Ok(data)
}
fn request(command: Command) -> guinea_service::Result<Request> {
    let operation = match command {
        Command::Capabilities => Operation::GetCapabilities {},
        Command::Files { command } => match command {
            Files::List { path, page } => Operation::ListFiles {
                path,
                offset: page.offset,
                limit: page.limit,
            },
            Files::Info { path } => Operation::GetFileInfo { path },
        },
        Command::Sessions { .. } => Operation::ListEditorSessions {},
        Command::Documents { command } => match command {
            Documents::Read {
                target,
                start,
                limit,
            } => Operation::ReadDocument {
                document: target.document,
                path: target.path,
                start,
                limit,
                expected_hash: target.expected_hash,
            },
            Documents::Search {
                needle,
                target,
                start,
                limit,
            } => Operation::SearchDocument {
                document: target.document,
                path: target.path,
                needle,
                start,
                limit,
                expected_hash: target.expected_hash,
            },
            Documents::Analyze { target, page } => Operation::AnalyzeDocument {
                document: target.document,
                path: target.path,
                offset: page.offset,
                limit: page.limit,
                expected_hash: target.expected_hash,
            },
            Documents::Open { path } => Operation::OpenDocument { path },
            Documents::New => Operation::NewDocument {},
            Documents::Save {
                document,
                expected_revision,
                path,
            } => Operation::SaveDocument {
                document,
                expected_revision,
                path,
            },
        },
        Command::Tabs { command } => match command {
            Tabs::List { page } => Operation::ListTabs {
                offset: page.offset,
                limit: page.limit,
            },
            Tabs::Activate { tab } => Operation::ActivateTab { tab },
            Tabs::Close { tab } => Operation::CloseTab { tab },
        },
        Command::Patch { command } => match command {
            Patch::Plan {
                document,
                expected_revision,
                edits_file,
            } => {
                let edits: Vec<PatchEdit> = serde_json::from_slice(&read_json(&edits_file)?)
                    .map_err(|e| Error::new("INVALID_ARGUMENT", e.to_string()))?;
                Operation::PlanDocumentPatch {
                    document,
                    expected_revision,
                    edits,
                }
            }
            Patch::Apply { plan } => Operation::ApplyDocumentPatch { plan },
        },
        Command::Review { document, split } => Operation::OpenReview { document, split },
        Command::Transaction { request_id } => Operation::GetTransaction {
            transaction_id: request_id,
        },
        Command::Request { file } => {
            return serde_json::from_slice(&read_json(&file)?)
                .map_err(|e| Error::new("INVALID_ARGUMENT", e.to_string()));
        }
    };
    Ok(Request::new(operation))
}
fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let fallback_id = cli.request_id.clone().unwrap_or_else(guinea_service::id);
    let fallback_session = cli.connect.session.clone();
    let result = (|| {
        let mut request = request(cli.command)?;
        if let Some(id) = cli.request_id {
            request.request_id = id;
        }
        let connection = Connection::new(&cli.connect)?;
        Ok::<_, Error>(connection.call(request, Arc::new(AtomicBool::new(false))))
    })();
    let value = match result {
        Ok(response) => serde_json::to_value(response).unwrap(),
        Err(error) => serde_json::to_value(guinea_service::Response {
            schema_version: 1,
            request_id: fallback_id,
            backend: if fallback_session.is_some() {
                "live"
            } else {
                "disk"
            }
            .into(),
            session_id: fallback_session,
            revision: None,
            error: Some(error),
            data: None,
        })
        .unwrap(),
    };
    println!("{}", serde_json::to_string(&value).unwrap());
    if value["error"].is_null() {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::from(1)
    }
}
