use super::*;
use crate::project::{Action as ProjectAction, Found, Mode, Project};
impl App {
    pub(super) fn open_project(&mut self, mode: Mode) {
        self.modal = Some(Modal::Project(Box::new(Project::new(
            self.explorer.root.clone(),
            mode,
        ))));
    }
    pub(super) fn project_key(&mut self, mut project: Box<Project>, key: KeyEvent) {
        match project.key(key) {
            ProjectAction::Close => self.message = "Cancelled".into(),
            ProjectAction::Open => {
                if let Some(found) = project.rows.get(project.selected).cloned() {
                    self.open_project_result(found);
                }
            }
            ProjectAction::None => self.modal = Some(Modal::Project(project)),
        }
    }
    pub(super) fn open_project_result(&mut self, found: Found) {
        if self.busy() {
            self.message = "Background operation in progress; retry when complete".into();
            return;
        }
        self.focus = Focus::Editor;
        self.project_jump = Some(found.clone());
        if let Some(index) = self.tabs.iter().position(|t| {
            t.review_source.is_none() && t.document.path.as_ref() == Some(&found.path)
        }) {
            self.active = index;
            self.apply_project_jump();
        } else {
            self.start_open(found.path, None, self.config.large_file_warning_bytes);
        }
    }
    pub(super) fn apply_project_jump(&mut self) {
        let Some(found) = self.project_jump.take() else {
            return;
        };
        if self.document().path.as_ref() != Some(&found.path) {
            return;
        }
        self.tabs[self.active].review.mode = ViewMode::Source;
        let Some(range) = found.range else {
            self.message = "Opened".into();
            return;
        };
        let document = self.document_mut();
        if document.is_dirty() || document.fingerprint.as_ref().map(|f| f.sha256) != found.hash {
            self.message = self
                .config
                .language
                .tr("Search result changed; current buffer retained. Find again in this tab.")
                .into_owned();
            return;
        }
        if document.select(range.start, range.end).is_ok() {
            self.tabs[self.active].follow_cursor = true;
            self.message = self.config.language.tr("Project match opened").into_owned();
        }
    }
    pub(super) fn project_mouse(&mut self, mouse: MouseEvent) {
        let Some(Modal::Project(mut project)) = self.modal.take() else {
            return;
        };
        let point = (mouse.column, mouse.row).into();
        if project.list.contains(point) {
            match mouse.kind {
                MouseEventKind::ScrollUp => project.move_by(-3),
                MouseEventKind::ScrollDown => project.move_by(3),
                MouseEventKind::Down(MouseButton::Left) if project.ready() => {
                    let at = project.top() + mouse.row.saturating_sub(project.list.y) as usize;
                    if let Some(found) = project.rows.get(at) {
                        let open = self.double_click(ModalClick::File(found.path.clone()));
                        project.selected = at;
                        if open {
                            let found = found.clone();
                            self.open_project_result(found);
                            return;
                        }
                    }
                }
                _ => {}
            }
        }
        let action = self
            .hits
            .modal_actions
            .iter()
            .find(|(r, _)| r.contains(point))
            .map(|(_, a)| *a);
        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            match action {
                Some("project-toggle") => project.toggle(),
                Some("project-open") if project.ready() => {
                    if let Some(found) = project.rows.get(project.selected).cloned() {
                        self.open_project_result(found);
                        return;
                    }
                }
                _ => {}
            }
        }
        self.modal = Some(Modal::Project(project));
    }
}
