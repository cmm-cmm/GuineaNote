use crate::{
    live::{Lease, Live, UiCommand, UiReply},
    policy::Policy,
    *,
};
use guinea_core::{Document, TextBuffer, TextEdit};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

struct Plan {
    document: String,
    revision: u64,
    prepared: Document,
    changed: bool,
}
pub struct Service {
    pub policy: Policy,
    pub live: Option<Live>,
    plans: HashMap<String, Plan>,
    completed: HashMap<String, (String, Response)>,
}
impl Service {
    pub fn new(policy: Policy, live: Option<Live>) -> Self {
        Self {
            policy,
            live,
            plans: HashMap::new(),
            completed: HashMap::new(),
        }
    }
    pub fn handle(&mut self, request: Request, cancelled: Arc<AtomicBool>) -> Response {
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&request.operation).unwrap())
        );
        if let Some((old, result)) = self.completed.get(&request.request_id) {
            if old == &fingerprint {
                return result.clone();
            }
            return self.response(
                &request.request_id,
                Err(Error::new(
                    "IDEMPOTENCY_CONFLICT",
                    "Request ID was already used with different arguments",
                )),
            );
        }
        let valid_id = !request.request_id.is_empty() && request.request_id.len() <= 128;
        let result = if !valid_id {
            Err(Error::new(
                "INVALID_ARGUMENT",
                "request_id must have 1..128 UTF-8 bytes",
            ))
        } else if cancelled.load(Ordering::Acquire) {
            Err(Error::new(
                "CANCELLED",
                "Request was cancelled before execution",
            ))
        } else if request.operation.mutates() && self.completed.len() >= 2048 {
            Err(Error::new(
                "LIMIT_EXCEEDED",
                "Session write ledger is full; restart the Agent session before more writes",
            ))
        } else {
            self.execute(request.operation.clone(), &cancelled)
        };
        let result = result.and_then(|value| {
            if serde_json::to_vec(&value).map_or(true, |v| v.len() > RESPONSE_BYTES) {
                Err(Error::new(
                    "LIMIT_EXCEEDED",
                    "Response exceeds 512 KiB; request a smaller page",
                ))
            } else {
                Ok(value)
            }
        });
        let response = self.response(
            if valid_id {
                &request.request_id
            } else {
                "invalid-request-id"
            },
            result,
        );
        if valid_id && request.operation.mutates() && self.completed.len() < 2048 {
            self.completed
                .insert(request.request_id, (fingerprint, response.clone()));
        }
        response
    }
    fn response(&self, id: &str, result: Result<Value>) -> Response {
        let revision = result
            .as_ref()
            .ok()
            .and_then(|v| v.get("revision"))
            .and_then(Value::as_u64);
        let (data, error) = match result {
            Ok(v) => (Some(v), None),
            Err(e) => (None, Some(e)),
        };
        Response {
            schema_version: 1,
            request_id: id.into(),
            backend: if self.live.is_some() { "live" } else { "disk" }.into(),
            session_id: self.live.as_ref().map(|l| l.session.clone()),
            revision,
            data,
            error,
        }
    }
    fn live(&self) -> Result<&Live> {
        self.live.as_ref().ok_or_else(|| {
            Error::new(
                "SESSION_REQUIRED",
                "This command requires an explicitly selected live TUI session",
            )
        })
    }
    fn snapshot(
        &self,
        document: Option<&str>,
        path: Option<&str>,
        action: &str,
    ) -> Result<(Document, Option<Lease>)> {
        match (document, path) {
            (Some(document), None) => {
                let (snapshot, lease) = self.live()?.acquire(document, action)?;
                for path in snapshot.path.iter().chain(snapshot.recovery_source.iter()) {
                    self.policy.relative(path)?;
                }
                if snapshot.buffer().len_bytes() as u64 > MAX_FILE {
                    return Err(Error::new(
                        "LIMIT_EXCEEDED",
                        "Live buffer exceeds the 10 MiB Agent budget",
                    ));
                }
                Ok((snapshot, Some(lease)))
            }
            (None, Some(path)) => {
                let snapshot = guinea_io::open(&self.policy.path(path, false)?, true, MAX_FILE)?;
                self.policy.relative(snapshot.path.as_ref().unwrap())?;
                Ok((snapshot, None))
            }
            _ => Err(Error::new(
                "INVALID_ARGUMENT",
                "Provide exactly one of document or path",
            )),
        }
    }
    fn tabs(&self) -> Result<Vec<live::TabInfo>> {
        match self.live()?.call(UiCommand::ListTabs)? {
            UiReply::Tabs(tabs) => Ok(tabs
                .into_iter()
                .filter(|t| {
                    t.path
                        .iter()
                        .chain(t.recovery_source.iter())
                        .all(|p| self.policy.relative(std::path::Path::new(p)).is_ok())
                })
                .collect()),
            _ => Err(Error::new("INTERNAL", "Unexpected tab reply")),
        }
    }
    fn tab(&self, id: &str) -> Result<u64> {
        if !self.tabs()?.iter().any(|t| t.tab == id) {
            return Err(Error::new("NOT_FOUND", "Tab not found in this workspace"));
        }
        self.live()?.parse_id(id, 't')
    }
    fn ui_value(&self, cancelled: &AtomicBool, command: UiCommand) -> Result<Value> {
        match self.live()?.call_cancelable(command, cancelled)? {
            UiReply::Value(v) => Ok(v),
            _ => Err(Error::new("INTERNAL", "Unexpected editor reply")),
        }
    }
    fn execute(&mut self, op: Operation, cancelled: &AtomicBool) -> Result<Value> {
        use Operation::*;
        if op.mutates() {
            self.policy.check_write()?;
            self.live()?;
        }
        match op {
            GetCapabilities {} => Ok(
                json!({"schema_version":1,"backend":if self.live.is_some(){"live"}else{"disk"},"root":self.policy.root,"access":if self.policy.write{"write"}else{"read-only"},"secret_read":self.policy.secrets,"coordinate_encoding":"unicode-scalar-zero-based-half-open-grapheme-boundaries","max_file_bytes":MAX_FILE,"page_bytes":PAGE_BYTES,"page_items":PAGE_ITEMS,"max_patch_bytes":1048576,"max_edits":100,"max_plans":32,"write_ledger_capacity":2048,"atomic_multi_file":false}),
            ),
            ListEditorSessions {} => Ok(
                json!({"sessions":crate::ipc::sessions()?.into_iter().filter(|s|s.root==self.policy.root).collect::<Vec<_>>()}),
            ),
            GetTransaction { transaction_id } => self
                .completed
                .get(&transaction_id)
                .map(|(_, r)| json!({"state":"completed","response":r}))
                .ok_or_else(|| {
                    Error::new(
                        "NOT_FOUND",
                        "No completed write with this request ID in this session",
                    )
                }),
            ListFiles {
                path,
                offset,
                limit,
            } => {
                page_limit(limit)?;
                let directory = self.policy.path(&path, false)?;
                let mut entries = Vec::new();
                let mut capped = false;
                for entry in fs::read_dir(directory)? {
                    if entries.len() >= 10000 {
                        capped = true;
                        break;
                    }
                    let entry = entry?;
                    let relative = entry
                        .path()
                        .strip_prefix(&self.policy.root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                    if self.policy.path(&relative, false).is_err() {
                        continue;
                    }
                    let meta = entry.metadata()?;
                    entries.push(
                        json!({"path":relative,"directory":meta.is_dir(),"bytes":meta.len()}),
                    );
                }
                entries.sort_by_key(|v| {
                    (
                        v["directory"] != true,
                        v["path"].as_str().unwrap().to_lowercase(),
                    )
                });
                let total = entries.len();
                Ok(
                    json!({"entries":entries.into_iter().skip(offset).take(limit).collect::<Vec<_>>(),"total":total,"directory_capped":capped,"truncated":offset.saturating_add(limit)<total||capped,"next_offset":(offset.saturating_add(limit)<total).then_some(offset.saturating_add(limit))}),
                )
            }
            GetFileInfo { path } => {
                let path = self.policy.path(&path, false)?;
                let meta = fs::metadata(&path)?;
                Ok(
                    json!({"path":self.policy.relative(&path)?,"bytes":meta.len(),"directory":meta.is_dir(),"readonly":meta.permissions().readonly(),"format":guinea_syntax::Language::detect(Some(&path)).label()}),
                )
            }
            ReadDocument {
                document,
                path,
                start,
                limit,
                expected_hash,
            } => {
                if limit == 0 || limit > PAGE_BYTES {
                    return Err(Error::new(
                        "LIMIT_EXCEEDED",
                        "Read limit must be 1..65536 bytes",
                    ));
                }
                let (snapshot, _lease) =
                    self.snapshot(document.as_deref(), path.as_deref(), "read")?;
                let hash = check_hash(&snapshot, expected_hash.as_deref())?;
                if start > snapshot.buffer().len_chars() || !snapshot.is_grapheme_boundary(start) {
                    return Err(Error::new("INVALID_ARGUMENT", "Invalid start boundary"));
                }
                let mut end = start
                    .saturating_add(limit)
                    .min(snapshot.buffer().len_chars());
                let mut text = snapshot.buffer().slice(start..end)?;
                if text.len() > limit {
                    end = start + text.char_indices().take_while(|(b, _)| *b < limit).count();
                    while snapshot.buffer().slice(start..end)?.len() > limit {
                        end -= 1;
                    }
                }
                while end > start && !snapshot.is_grapheme_boundary(end) {
                    end -= 1;
                }
                if end == start && start < snapshot.buffer().len_chars() {
                    return Err(Error::new("LIMIT_EXCEEDED", "Grapheme exceeds page budget"));
                }
                text = snapshot.buffer().slice(start..end)?;
                Ok(
                    json!({"source":if document.is_some(){"live"}else{"disk"},"document":document,"revision":snapshot.revision(),"hash":hash,"start":start,"end":end,"text":text,"encoding":"UTF-8","bom":snapshot.bom,"line_endings":snapshot.line_ending().label(),"truncated":end<snapshot.buffer().len_chars(),"next_start":(end<snapshot.buffer().len_chars()).then_some(end)}),
                )
            }
            SearchDocument {
                document,
                path,
                needle,
                start,
                limit,
                expected_hash,
            } => {
                page_limit(limit)?;
                if needle.is_empty() || needle.len() > PAGE_BYTES {
                    return Err(Error::new(
                        "INVALID_ARGUMENT",
                        "Search needle must have 1..65536 bytes",
                    ));
                }
                let (snapshot, _lease) =
                    self.snapshot(document.as_deref(), path.as_deref(), "search")?;
                let hash = check_hash(&snapshot, expected_hash.as_deref())?;
                let text = snapshot.text();
                if start > snapshot.buffer().len_chars() || !snapshot.is_grapheme_boundary(start) {
                    return Err(Error::new("INVALID_ARGUMENT", "Invalid start boundary"));
                }
                let mut matches = Vec::new();
                let mut next = None;
                let mut previous_byte = 0;
                let mut scalar = 0;
                for (byte, matched) in text.match_indices(&needle) {
                    if cancelled.load(Ordering::Acquire) {
                        return Err(Error::new("CANCELLED", "Search cancelled"));
                    }
                    scalar += text[previous_byte..byte].chars().count();
                    previous_byte = byte;
                    let end = scalar + matched.chars().count();
                    if scalar >= start
                        && snapshot.is_grapheme_boundary(scalar)
                        && snapshot.is_grapheme_boundary(end)
                    {
                        if matches.len() == limit {
                            next = Some(scalar);
                            break;
                        }
                        matches.push(json!({"start":scalar,"end":end}));
                    }
                }
                Ok(
                    json!({"source":if document.is_some(){"live"}else{"disk"},"document":document,"revision":snapshot.revision(),"hash":hash,"matches":matches,"truncated":next.is_some(),"next_start":next}),
                )
            }
            AnalyzeDocument {
                document,
                path,
                offset,
                limit,
                expected_hash,
            } => {
                let (snapshot, _lease) =
                    self.snapshot(document.as_deref(), path.as_deref(), "analyze")?;
                let hash = check_hash(&snapshot, expected_hash.as_deref())?;
                let mut result = analysis::analyze(
                    &snapshot.text(),
                    guinea_syntax::Language::detect(
                        snapshot
                            .path
                            .as_deref()
                            .or(snapshot.recovery_source.as_deref()),
                    ),
                    offset,
                    limit,
                )?;
                result["revision"] = snapshot.revision().into();
                result["hash"] = hash.into();
                result["source"] = if document.is_some() { "live" } else { "disk" }.into();
                result["document"] = json!(document);
                if cancelled.load(Ordering::Acquire) {
                    return Err(Error::new("CANCELLED", "Analysis cancelled"));
                }
                Ok(result)
            }
            ListTabs { offset, limit } => {
                page_limit(limit)?;
                let tabs = self.tabs()?;
                let total = tabs.len();
                Ok(
                    json!({"tabs":tabs.into_iter().skip(offset).take(limit).collect::<Vec<_>>(),"total":total,"truncated":offset.saturating_add(limit)<total,"next_offset":(offset.saturating_add(limit)<total).then_some(offset.saturating_add(limit))}),
                )
            }
            ActivateTab { tab } => self.ui_value(
                cancelled,
                UiCommand::Activate {
                    tab: self.tab(&tab)?,
                },
            ),
            NewDocument {} => self.ui_value(cancelled, UiCommand::New),
            OpenDocument { path } => {
                let document = guinea_io::open(&self.policy.path(&path, true)?, false, MAX_FILE)?;
                self.policy.relative(document.path.as_ref().unwrap())?;
                self.ui_value(cancelled, UiCommand::Open { document })
            }
            PlanDocumentPatch {
                document,
                expected_revision,
                edits,
            } => {
                if edits.is_empty()
                    || edits.len() > 100
                    || edits.iter().map(|e| e.text.len()).sum::<usize>() > 1048576
                {
                    return Err(Error::new(
                        "LIMIT_EXCEEDED",
                        "Patch needs 1..100 edits and <=1 MiB inserted text",
                    ));
                }
                if edits.iter().any(|e| e.text.contains('\0')) {
                    return Err(Error::new(
                        "INVALID_ARGUMENT",
                        "Patch text cannot contain NUL",
                    ));
                }
                if self.plans.len() >= 32 {
                    return Err(Error::new(
                        "LIMIT_EXCEEDED",
                        "At most 32 pending plans per session",
                    ));
                }
                let (mut snapshot, _lease) = self.snapshot(Some(&document), None, "plan patch")?;
                if snapshot.revision() != expected_revision {
                    return Err(Error::new(
                        "REVISION_CONFLICT",
                        "Document changed since it was read",
                    ));
                }
                let original = snapshot.review_snapshot();
                let edits = edits
                    .into_iter()
                    .map(|e| TextEdit {
                        start: e.start,
                        end: e.end,
                        text: e.text,
                    })
                    .collect::<Vec<_>>();
                snapshot.apply_edits(&edits)?;
                let mut diff = String::new();
                for edit in &edits {
                    let old = original
                        .buffer()
                        .slice(edit.start..edit.start.saturating_add(512).min(edit.end))?;
                    diff.push_str(&format!(
                        "@@ scalar {}..{} @@\n-{}\n+{}\n",
                        edit.start,
                        edit.end,
                        old.chars().take(512).collect::<String>(),
                        edit.text.chars().take(512).collect::<String>()
                    ));
                }
                if snapshot.buffer().len_bytes() as u64 > MAX_FILE {
                    return Err(Error::new(
                        "LIMIT_EXCEEDED",
                        "Patched buffer would exceed 10 MiB",
                    ));
                }
                if cancelled.load(Ordering::Acquire) {
                    return Err(Error::new("CANCELLED", "Plan cancelled before publication"));
                }
                let plan = id();
                self.ui_value(cancelled, UiCommand::Preview {
                    title: format!("AI diff {}", &plan[..8]),
                    text: format!("AI PROPOSED CHANGES · EXCERPT\nSource is unchanged until apply. Ranges use original Unicode scalar offsets.\n\n{}",diff.chars().take(PAGE_BYTES).collect::<String>()),
                })?;
                self.plans.insert(
                    plan.clone(),
                    Plan {
                        document: document.clone(),
                        revision: expected_revision,
                        changed: snapshot.revision() != expected_revision,
                        prepared: snapshot,
                    },
                );
                Ok(
                    json!({"plan":plan,"document":document,"revision":expected_revision,"applied":false,"preview":"AI diff tab","diff_is_excerpt":true}),
                )
            }
            ApplyDocumentPatch { plan } => {
                let plan_id = plan;
                let plan = self.plans.get(&plan_id).ok_or_else(|| {
                    Error::new("NOT_FOUND", "Patch plan is missing or already applied")
                })?;
                let (snapshot, lease) = self.snapshot(Some(&plan.document), None, "apply patch")?;
                if snapshot.readonly {
                    return Err(Error::new("READ_ONLY", "Document is read-only"));
                }
                if snapshot.revision() != plan.revision {
                    return Err(Error::new(
                        "REVISION_CONFLICT",
                        "Patch base revision is stale",
                    ));
                }
                if !plan.changed {
                    let result = json!({"document":plan.document,"revision":plan.revision,"applied":false,"changed":false});
                    self.plans.remove(&plan_id);
                    return Ok(result);
                }
                if cancelled.load(Ordering::Acquire) {
                    return Err(Error::new("CANCELLED", "Patch cancelled before commit"));
                }
                let value = self.ui_value(
                    cancelled,
                    UiCommand::Apply {
                        document: snapshot.id,
                        lease: lease.as_ref().unwrap().id.clone(),
                        expected_revision: plan.revision,
                        prepared: plan.prepared.review_snapshot(),
                    },
                )?;
                self.plans.remove(&plan_id);
                Ok(value)
            }
            SaveDocument {
                document,
                expected_revision,
                path,
            } => {
                let (mut snapshot, lease) = self.snapshot(Some(&document), None, "save")?;
                if snapshot.revision() != expected_revision {
                    return Err(Error::new("REVISION_CONFLICT", "Save revision is stale"));
                }
                let target = match path {
                    Some(path) => self.policy.path(&path, true)?,
                    None => snapshot.path.clone().ok_or_else(|| {
                        Error::new("INVALID_ARGUMENT", "Blank document needs a save path")
                    })?,
                };
                self.policy.relative(&target)?;
                if cancelled.load(Ordering::Acquire) {
                    return Err(Error::new("CANCELLED", "Save cancelled before I/O"));
                }
                let report = guinea_io::save(&mut snapshot, &target)?;
                let mut value = self.ui_value(
                    cancelled,
                    UiCommand::Saved {
                        document: snapshot.id,
                        lease: lease.as_ref().unwrap().id.clone(),
                        expected_revision,
                        saved: snapshot,
                    },
                )?;
                value["warning"] = json!(report.warning);
                Ok(value)
            }
            OpenReview { document, split } => {
                let (snapshot, lease) = self.snapshot(Some(&document), None, "review")?;
                drop(lease);
                self.ui_value(
                    cancelled,
                    UiCommand::Review {
                        document: snapshot.id,
                        split,
                    },
                )
            }
            CloseTab { tab } => self.ui_value(
                cancelled,
                UiCommand::Close {
                    tab: self.tab(&tab)?,
                },
            ),
        }
    }
}
fn page_limit(limit: usize) -> Result<()> {
    if limit == 0 || limit > PAGE_ITEMS {
        Err(Error::new("LIMIT_EXCEEDED", "Page limit must be 1..200"))
    } else {
        Ok(())
    }
}
fn check_hash(document: &Document, expected: Option<&str>) -> Result<String> {
    let mut hasher = Sha256::new();
    document
        .buffer()
        .visit_chunks(0..document.buffer().len_chars(), &mut |chunk| {
            hasher.update(chunk);
            true
        })?;
    let hash = format!("{:x}", hasher.finalize());
    if expected.is_some_and(|s| s != hash) {
        return Err(Error::new(
            "REVISION_CONFLICT",
            "Content hash changed between pages",
        ));
    }
    Ok(hash)
}
