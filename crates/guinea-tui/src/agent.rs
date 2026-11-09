//! Adapter running only on the TUI state-owner thread. Snapshot preparation and
//! every filesystem/parser operation belong to guinea-service workers.
use super::*;
use guinea_service::{
    Error, Result,
    live::{TabInfo, UiCommand, UiReply},
};
use serde_json::json;
use std::{
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};
pub(super) struct LeaseState {
    id: String,
    readonly: bool,
}
impl App {
    pub fn enable_agent(
        &mut self,
        write: bool,
        secrets: bool,
        wake: Option<Arc<dyn Fn() + Send + Sync>>,
    ) -> Result<()> {
        let policy = guinea_service::policy::Policy::new(
            &self.explorer.root,
            write && !self.readonly,
            secrets,
        )?;
        let bridge = guinea_service::ipc::Bridge::start(policy, wake)?;
        self.message = format!(
            "Agent session: {} · {}",
            bridge.session,
            if write && !self.readonly {
                "write"
            } else {
                "read-only"
            }
        );
        self.agent = Some(bridge);
        self.redraw = true;
        Ok(())
    }
    pub(super) fn poll_agent(&mut self) {
        let started = Instant::now();
        for _ in 0..4 {
            if started.elapsed() > Duration::from_millis(2) {
                break;
            }
            let call = self
                .agent
                .as_ref()
                .and_then(|bridge| bridge.receiver.try_recv().ok());
            let Some(call) = call else {
                break;
            };
            let acquired = matches!(call.command, UiCommand::Acquire { .. });
            if call.cancelled.load(Ordering::Acquire)
                && !matches!(
                    call.command,
                    UiCommand::Release { .. } | UiCommand::Saved { .. }
                )
            {
                let _ = call.reply.send(Err(Error::new(
                    "CANCELLED",
                    "UI request cancelled before commit",
                )));
                continue;
            }
            let result = self.agent_command(call.command);
            let _ = call.reply.send(result);
            // Paint the AI/read-only state before a very short worker can
            // enqueue its release in the same event-loop iteration.
            if acquired {
                break;
            }
        }
    }
    fn source_index(&self, id: u64) -> Result<usize> {
        self.tabs
            .iter()
            .position(|t| t.review_source.is_none() && t.document.id == id)
            .ok_or_else(|| Error::new("NOT_FOUND", "Document is no longer open"))
    }
    fn lease_index(&self, document: u64, lease: &str) -> Result<usize> {
        if !self
            .agent_leases
            .get(&document)
            .is_some_and(|s| s.id == lease)
        {
            return Err(Error::new(
                "REVISION_CONFLICT",
                "AI lease expired or does not own this document",
            ));
        }
        self.source_index(document)
    }
    fn agent_value(&self, index: usize) -> UiReply {
        let live = &self.agent.as_ref().unwrap().live;
        let tab = &self.tabs[index];
        UiReply::Value(
            json!({"tab":live.tab_id(tab.id),"document":live.document_id(tab.document.id),"revision":tab.document.revision(),"dirty":tab.document.is_dirty()}),
        )
    }
    fn agent_command(&mut self, command: UiCommand) -> Result<UiReply> {
        if self.busy()
            && !matches!(
                command,
                UiCommand::Release { .. } | UiCommand::ListTabs | UiCommand::Saved { .. }
            )
        {
            return Err(Error::new("BUSY", "Editor filesystem operation is running"));
        }
        if self.modal.is_some()
            && !matches!(
                command,
                UiCommand::Release { .. } | UiCommand::ListTabs | UiCommand::Saved { .. }
            )
        {
            return Err(Error::new(
                "BUSY",
                "Finish the current editor dialog before Agent operations",
            ));
        }
        match command {
            UiCommand::ListTabs => {
                let live = &self.agent.as_ref().unwrap().live;
                Ok(UiReply::Tabs(
                    self.tabs
                        .iter()
                        .enumerate()
                        .map(|(index, t)| TabInfo {
                            tab: live.tab_id(t.id),
                            document: live.document_id(t.document.id),
                            title: t.title(self.config.language),
                            path: t
                                .document
                                .path
                                .as_ref()
                                .map(|p| p.to_string_lossy().into_owned()),
                            revision: t.document.revision(),
                            dirty: t.document.is_dirty(),
                            readonly: self
                                .agent_leases
                                .get(&t.document.id)
                                .map_or(t.document.readonly, |s| s.readonly),
                            review: t.review_source.is_some(),
                            ai: t.ai_action.is_some(),
                            active: index == self.active,
                        })
                        .collect(),
                ))
            }
            UiCommand::Acquire {
                document,
                lease,
                action,
            } => {
                let index = self.source_index(document)?;
                if self.agent_leases.contains_key(&document) {
                    return Err(Error::new("BUSY", "Document is already leased by AI"));
                }
                let readonly = self.tabs[index].document.readonly;
                let mut snapshot = self.tabs[index].document.review_snapshot();
                snapshot.readonly = readonly;
                self.agent_leases.insert(
                    document,
                    LeaseState {
                        id: lease,
                        readonly,
                    },
                );
                for tab in &mut self.tabs {
                    if tab.document.id == document {
                        tab.ai_action = Some(action.clone());
                        tab.document.readonly = true;
                    }
                }
                self.redraw = true;
                Ok(UiReply::Document(snapshot))
            }
            UiCommand::Release { lease } => {
                if let Some(document) = self
                    .agent_leases
                    .iter()
                    .find(|(_, s)| s.id == lease)
                    .map(|(id, _)| *id)
                {
                    let state = self.agent_leases.remove(&document).unwrap();
                    for tab in &mut self.tabs {
                        if tab.document.id == document {
                            tab.ai_action = None;
                            tab.document.readonly = if tab.review_source.is_some() {
                                true
                            } else {
                                state.readonly
                            };
                        }
                    }
                    self.redraw = true;
                }
                Ok(UiReply::Value(json!({"released":true})))
            }
            UiCommand::Activate { tab } => {
                self.active = self
                    .tabs
                    .iter()
                    .position(|t| t.id == tab)
                    .ok_or_else(|| Error::new("NOT_FOUND", "Tab was closed"))?;
                self.focus = Focus::Editor;
                self.redraw = true;
                Ok(self.agent_value(self.active))
            }
            UiCommand::New => {
                self.new_blank();
                self.redraw = true;
                Ok(self.agent_value(self.active))
            }
            UiCommand::Open { mut document } => {
                if let Some(index) = self
                    .tabs
                    .iter()
                    .position(|t| t.review_source.is_none() && t.document.path == document.path)
                {
                    self.active = index;
                } else {
                    document.readonly |= self.readonly;
                    self.tabs.push(Tab::new(document));
                    self.active = self.tabs.len() - 1;
                }
                self.focus = Focus::Editor;
                self.redraw = true;
                Ok(self.agent_value(self.active))
            }
            UiCommand::Apply {
                document,
                lease,
                expected_revision,
                prepared,
            } => {
                let index = self.lease_index(document, &lease)?;
                let readonly = self.agent_leases[&document].readonly;
                self.tabs[index].document.readonly = readonly;
                let result = self.tabs[index]
                    .document
                    .accept_prepared(&prepared, expected_revision);
                self.tabs[index].document.readonly = true;
                result?;
                self.redraw = true;
                Ok(self.agent_value(index))
            }
            UiCommand::Saved {
                document,
                lease,
                expected_revision,
                saved,
            } => {
                let index = self.lease_index(document, &lease)?;
                if self.tabs[index].document.revision() != expected_revision {
                    return Err(Error::new(
                        "REVISION_CONFLICT",
                        "Saved snapshot revision no longer matches; inspect disk before retrying",
                    ));
                }
                self.tabs[index].document.accept_saved_snapshot(&saved);
                self.redraw = true;
                Ok(self.agent_value(index))
            }
            UiCommand::Review { document, split } => {
                self.active = self.source_index(document)?;
                if split {
                    self.tabs[self.active].review.mode = ViewMode::Split;
                } else {
                    self.open_review_tab();
                }
                self.redraw = true;
                Ok(self.agent_value(self.active))
            }
            UiCommand::Preview { title, text } => {
                let mut document = Document::from_text(&text);
                document.readonly = true;
                let mut tab = Tab::new(document);
                tab.title_override = Some(title);
                self.tabs.push(tab);
                self.redraw = true;
                Ok(self.agent_value(self.tabs.len() - 1))
            }
            UiCommand::Close { tab } => {
                let index = self
                    .tabs
                    .iter()
                    .position(|t| t.id == tab)
                    .ok_or_else(|| Error::new("NOT_FOUND", "Tab was closed"))?;
                if self.tabs[index].ai_action.is_some() {
                    return Err(Error::new("BUSY", "AI is working in this tab"));
                }
                if self.tabs[index].review_source.is_none() && self.tabs[index].document.is_dirty()
                {
                    return Err(Error::new(
                        "DIRTY_DOCUMENT",
                        "Save or explicitly close the dirty tab in the TUI",
                    ));
                }
                self.close_at(index);
                self.redraw = true;
                Ok(UiReply::Value(json!({"closed":true})))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guinea_core::TextEdit;
    fn setup(readonly: bool) -> (App, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let mut app = App::new(
            vec![directory.path().to_path_buf()],
            readonly,
            Config::default(),
        );
        app.focus = Focus::Editor;
        app.agent = Some(
            guinea_service::ipc::Bridge::start_in(
                guinea_service::policy::Policy::new(directory.path(), !readonly, false).unwrap(),
                None,
                directory.path().join("runtime"),
            )
            .unwrap(),
        );
        (app, directory)
    }
    #[test]
    fn ai_lease_locks_source_and_review_and_keeps_other_tabs_editable() {
        let (mut a, _directory) = setup(false);
        a.document_mut().insert("original").unwrap();
        let source = a.document().id;
        a.open_review_tab();
        a.source_tab();
        let UiReply::Document(mut prepared) = a
            .agent_command(UiCommand::Acquire {
                document: source,
                lease: "lease".into(),
                action: "plan".into(),
            })
            .unwrap()
        else {
            panic!("no snapshot")
        };
        assert!(!prepared.readonly);
        for tab in &a.tabs {
            assert!(tab.ai_action.is_some());
            assert!(tab.document.readonly);
        }
        a.handle(Event::Paste("blocked".into()));
        a.command("undo");
        a.command("save");
        a.command("close");
        assert_eq!(a.document().text(), "original");
        assert!(a.modal.is_none());
        a.command("new");
        a.handle(Event::Paste("other tab".into()));
        let other = a.document().id;
        prepared
            .apply_edits(&[TextEdit {
                start: 0,
                end: 8,
                text: "AI result".into(),
            }])
            .unwrap();
        a.agent_command(UiCommand::Apply {
            document: source,
            lease: "lease".into(),
            expected_revision: 1,
            prepared,
        })
        .unwrap();
        assert_eq!(a.document().id, other);
        a.agent_command(UiCommand::Release {
            lease: "lease".into(),
        })
        .unwrap();
        assert!(!a.tabs[0].document.readonly);
        assert!(a.tabs[1].document.readonly);
        assert!(a.tabs.iter().all(|t| t.ai_action.is_none()));
        a.active = 0;
        a.command("undo");
        assert_eq!(a.document().text(), "original");
    }
    #[test]
    fn lease_drop_after_worker_panic_restores_original_readonly_state() {
        for readonly in [false, true] {
            let (mut a, _directory) = setup(readonly);
            let source = a.document().id;
            let live = a.agent.as_ref().unwrap().live.clone();
            let document = live.document_id(source);
            let worker = std::thread::spawn(move || {
                let (_snapshot, _lease) = live.acquire(&document, "test").unwrap();
                panic!("intentional Agent failure");
            });
            let deadline = Instant::now() + Duration::from_secs(5);
            while !worker.is_finished() {
                a.poll_agent();
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(worker.join().is_err());
            a.poll_agent();
            assert!(a.agent_leases.is_empty());
            assert_eq!(a.document().readonly, readonly);
        }
    }
}
