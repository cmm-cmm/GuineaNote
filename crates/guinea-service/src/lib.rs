//! Transport-independent Agent contract. File contents never become commands.
pub mod analysis;
pub mod ipc;
pub mod live;
pub mod policy;
mod service;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use service::Service;

pub const PAGE_BYTES: usize = 65_536;
pub const MAX_FILE: u64 = 10 * 1024 * 1024;
pub const MAX_MESSAGE: usize = 2 * 1024 * 1024;
pub const PAGE_ITEMS: usize = 200;
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PatchEdit {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    GetCapabilities {},
    ListFiles {
        #[serde(default)]
        path: String,
        #[serde(default)]
        offset: usize,
        #[serde(default = "page_items")]
        limit: usize,
    },
    GetFileInfo {
        path: String,
    },
    ReadDocument {
        #[serde(default)]
        document: Option<String>,
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        start: usize,
        #[serde(default = "page_bytes")]
        limit: usize,
        #[serde(default)]
        expected_hash: Option<String>,
    },
    SearchDocument {
        #[serde(default)]
        document: Option<String>,
        #[serde(default)]
        path: Option<String>,
        needle: String,
        #[serde(default)]
        start: usize,
        #[serde(default = "page_items")]
        limit: usize,
        #[serde(default)]
        expected_hash: Option<String>,
    },
    AnalyzeDocument {
        #[serde(default)]
        document: Option<String>,
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        offset: usize,
        #[serde(default = "page_items")]
        limit: usize,
        #[serde(default)]
        expected_hash: Option<String>,
    },
    ListEditorSessions {},
    ListTabs {
        #[serde(default)]
        offset: usize,
        #[serde(default = "page_items")]
        limit: usize,
    },
    ActivateTab {
        tab: String,
    },
    OpenDocument {
        path: String,
    },
    NewDocument {},
    PlanDocumentPatch {
        document: String,
        expected_revision: u64,
        edits: Vec<PatchEdit>,
    },
    ApplyDocumentPatch {
        plan: String,
    },
    SaveDocument {
        document: String,
        expected_revision: u64,
        #[serde(default)]
        path: Option<String>,
    },
    OpenReview {
        document: String,
        #[serde(default)]
        split: bool,
    },
    CloseTab {
        tab: String,
    },
    GetTransaction {
        transaction_id: String,
    },
}
fn page_items() -> usize {
    PAGE_ITEMS
}
fn page_bytes() -> usize {
    PAGE_BYTES
}
impl Operation {
    pub fn mutates(&self) -> bool {
        matches!(
            self,
            Self::OpenDocument { .. }
                | Self::NewDocument {}
                | Self::PlanDocumentPatch { .. }
                | Self::ApplyDocumentPatch { .. }
                | Self::SaveDocument { .. }
                | Self::OpenReview { .. }
                | Self::CloseTab { .. }
        )
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::GetCapabilities {} => "get_capabilities",
            Self::ListFiles { .. } => "list_files",
            Self::GetFileInfo { .. } => "get_file_info",
            Self::ReadDocument { .. } => "read_document",
            Self::SearchDocument { .. } => "search_document",
            Self::AnalyzeDocument { .. } => "analyze_document",
            Self::ListEditorSessions {} => "list_editor_sessions",
            Self::ListTabs { .. } => "list_tabs",
            Self::ActivateTab { .. } => "activate_tab",
            Self::OpenDocument { .. } => "open_document",
            Self::NewDocument {} => "new_document",
            Self::PlanDocumentPatch { .. } => "plan_document_patch",
            Self::ApplyDocumentPatch { .. } => "apply_document_patch",
            Self::SaveDocument { .. } => "save_document",
            Self::OpenReview { .. } => "open_review",
            Self::CloseTab { .. } => "close_tab",
            Self::GetTransaction { .. } => "get_transaction",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub request_id: String,
    #[serde(flatten)]
    pub operation: Operation,
}
impl Request {
    pub fn new(operation: Operation) -> Self {
        Self {
            request_id: id(),
            operation,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub code: String,
    pub message: String,
}
impl Error {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::new(
            "IO_ERROR",
            format!(
                "Filesystem or transport operation failed ({:?}, {:?})",
                error.kind(),
                error.raw_os_error()
            ),
        )
    }
}
impl From<guinea_io::FileError> for Error {
    fn from(e: guinea_io::FileError) -> Self {
        use guinea_io::FileError::*;
        Self::new(
            match &e {
                Conflict | DestinationExists => "DISK_CONFLICT",
                ReadOnly => "READ_ONLY",
                LargeFile { .. } => "LIMIT_EXCEEDED",
                SpecialFile => "OUT_OF_SCOPE",
                _ => "IO_ERROR",
            },
            e.to_string(),
        )
    }
}
impl From<guinea_core::EditError> for Error {
    fn from(e: guinea_core::EditError) -> Self {
        Self::new(
            match e {
                guinea_core::EditError::ReadOnly => "READ_ONLY",
                guinea_core::EditError::RevisionConflict => "REVISION_CONFLICT",
                _ => "INVALID_ARGUMENT",
            },
            e.to_string(),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub schema_version: u32,
    pub request_id: String,
    pub backend: String,
    pub session_id: Option<String>,
    pub revision: Option<u64>,
    pub data: Option<Value>,
    pub error: Option<Error>,
}
impl Response {
    pub fn result(&self) -> Result<Value> {
        self.error
            .clone()
            .map_or_else(|| Ok(self.data.clone().unwrap_or(Value::Null)), Err)
    }
}

pub fn tool_schemas() -> Vec<(String, Value)> {
    let schema = serde_json::to_value(schemars::schema_for!(Operation)).expect("schema serializes");
    schema["oneOf"]
        .as_array()
        .expect("tagged operations")
        .iter()
        .map(|variant| {
            let mut input = variant.clone();
            let name = input["properties"]["op"]["const"]
                .as_str()
                .expect("tag")
                .to_owned();
            input["properties"].as_object_mut().unwrap().remove("op");
            if let Some(required) = input["required"].as_array_mut() {
                required.retain(|v| v != "op");
            }
            input["$defs"] = schema["$defs"].clone();
            (name, input)
        })
        .collect()
}
