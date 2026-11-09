use clap::{Args, ValueEnum};
use guinea_service::{
    Error, Operation, Request, Response, Result, Service, ipc::Client, policy::Policy,
};
use std::{
    path::PathBuf,
    sync::{Mutex, atomic::AtomicBool},
};
#[derive(Clone, Copy, ValueEnum, Default)]
pub enum Access {
    #[default]
    ReadOnly,
    Write,
}
#[derive(Args, Clone)]
pub struct ConnectArgs {
    #[arg(long, global = true, default_value = ".")]
    pub root: PathBuf,
    #[arg(long, global = true)]
    pub session: Option<String>,
    #[arg(long, global = true, value_enum, default_value = "read-only")]
    pub access: Access,
    #[arg(long, global = true)]
    pub allow_secrets: bool,
}
enum Backend {
    Disk(Mutex<Service>),
    Live(Client),
}
pub struct Connection {
    backend: Backend,
    pub write: bool,
}
impl Connection {
    pub fn new(args: &ConnectArgs) -> Result<Self> {
        let write = matches!(args.access, Access::Write);
        let backend = if let Some(session) = &args.session {
            let client = Client::connect(session)?;
            if args.root.canonicalize()? != client.info().root {
                return Err(Error::new(
                    "OUT_OF_SCOPE",
                    "Selected editor session belongs to a different workspace; pass its --root explicitly",
                ));
            }
            if write && !client.info().write {
                return Err(Error::new(
                    "READ_ONLY",
                    "TUI session has not granted writes",
                ));
            }
            Backend::Live(client)
        } else {
            if write {
                return Err(Error::new(
                    "SESSION_REQUIRED",
                    "Writes require a live session; disk mode is read-only",
                ));
            }
            Backend::Disk(Mutex::new(Service::new(
                Policy::new(&args.root, false, args.allow_secrets)?,
                None,
            )))
        };
        Ok(Self { backend, write })
    }
    pub fn call(&self, request: Request, cancelled: std::sync::Arc<AtomicBool>) -> Response {
        let valid_id = !request.request_id.is_empty() && request.request_id.len() <= 128;
        let request_id = if valid_id {
            request.request_id.clone()
        } else {
            "invalid-request-id".into()
        };
        let result = if !valid_id {
            Err(Error::new(
                "INVALID_ARGUMENT",
                "request_id must have 1..128 UTF-8 bytes",
            ))
        } else if request.operation.mutates() && !self.write {
            Err(Error::new(
                "READ_ONLY",
                "This connection is read-only; writes need explicit grants from both client and TUI",
            ))
        } else {
            match &self.backend {
                Backend::Disk(service) => service
                    .lock()
                    .map(|mut s| s.handle(request, cancelled.clone()))
                    .map_err(|_| Error::new("INTERNAL", "Service worker stopped")),
                Backend::Live(client) => client.call(request, &cancelled),
            }
        };
        match result {
            Ok(mut response) => {
                if let Some(data) = &mut response.data
                    && data.get("access").is_some()
                {
                    data["access"] = if self.write { "write" } else { "read-only" }.into();
                }
                response
            }
            Err(error) => Response {
                schema_version: 1,
                request_id,
                backend: if self.live() { "live" } else { "disk" }.into(),
                session_id: self.session(),
                revision: None,
                data: None,
                error: Some(error),
            },
        }
    }
    pub fn live(&self) -> bool {
        matches!(&self.backend, Backend::Live(_))
    }
    pub fn session(&self) -> Option<String> {
        match &self.backend {
            Backend::Live(c) => Some(c.info().session),
            _ => None,
        }
    }
}
pub fn tool_write(name: &str) -> bool {
    matches!(
        name,
        "open_document"
            | "new_document"
            | "plan_document_patch"
            | "apply_document_patch"
            | "save_document"
            | "open_review"
            | "close_tab"
    )
}
pub fn operation(
    name: &str,
    mut arguments: serde_json::Map<String, serde_json::Value>,
) -> Result<Operation> {
    if arguments.contains_key("op") {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Tool arguments cannot override operation",
        ));
    }
    arguments.insert("op".into(), name.into());
    serde_json::from_value(arguments.into())
        .map_err(|e| Error::new("INVALID_ARGUMENT", e.to_string()))
}
