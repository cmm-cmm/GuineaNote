use crate::{app::document_language, i18n::Locale};
use guinea_core::{Document, TextBuffer};
use std::{
    sync::mpsc::{self, Receiver},
    thread,
    time::SystemTime,
};
use unicode_segmentation::UnicodeSegmentation;

pub struct Field {
    pub en: &'static str,
    pub vi: &'static str,
    pub value: String,
}
pub struct Details {
    pub name: String,
    pub language: &'static str,
    pub size: usize,
    pub lines: usize,
    pub dirty: bool,
    pub fields: Vec<Field>,
}
pub struct FileInfo {
    pub details: Option<Details>,
    pub top: usize,
    pub height: usize,
    pub error: Option<String>,
    pending: Option<Receiver<Details>>,
}
fn timestamp(value: std::io::Result<SystemTime>) -> String {
    value
        .ok()
        .and_then(|value| {
            time::OffsetDateTime::from(value)
                .format(&time::format_description::well_known::Rfc3339)
                .ok()
        })
        .unwrap_or_else(|| "—".into())
}
pub fn bytes(size: usize) -> String {
    if size >= 1024 * 1024 {
        format!("{:.2} MiB", size as f64 / (1024. * 1024.))
    } else if size >= 1024 {
        format!("{:.2} KiB", size as f64 / 1024.)
    } else {
        format!("{size} B")
    }
}
fn inspect(document: &Document, locale: Locale, history: guinea_core::HistoryStatus) -> Details {
    let yes = if locale == Locale::Vi { "Có" } else { "Yes" };
    let no = if locale == Locale::Vi { "Không" } else { "No" };
    let text = document.text();
    let mut fields = Vec::new();
    let mut add = |en, vi, value| fields.push(Field { en, vi, value });
    if let Some(source) = &document.recovery_source {
        add("Recovered from", "Khôi phục từ", crate::paths::text(source));
    }
    add(
        "Full path",
        "Đường dẫn",
        document
            .path
            .as_deref()
            .map(crate::paths::text)
            .unwrap_or_else(|| "—".into()),
    );
    add(
        "Buffer bytes",
        "Byte trong bộ đệm",
        document.buffer().len_bytes().to_string(),
    );
    add(
        "Characters",
        "Ký tự Unicode",
        document.buffer().len_chars().to_string(),
    );
    add(
        "Graphemes",
        "Cụm ký tự",
        text.graphemes(true).count().to_string(),
    );
    add("Words", "Số từ", text.unicode_words().count().to_string());
    add(
        "Encoding",
        "Mã hóa",
        if document.bom { "UTF-8 + BOM" } else { "UTF-8" }.into(),
    );
    add(
        "Line endings",
        "Xuống dòng",
        document.line_ending().label().into(),
    );
    add(
        "Editor read-only",
        "Chỉ đọc trong editor",
        if document.readonly { yes } else { no }.into(),
    );
    add(
        "Undo / Redo steps",
        "Số bước Hoàn tác / Làm lại",
        format!("{} / {}", history.undo, history.redo),
    );
    add(
        "History charged bytes",
        "Byte tính cho lịch sử",
        history.charged_bytes.to_string(),
    );
    add(
        "Trimmed Undo steps",
        "Bước Hoàn tác cũ đã bỏ",
        history.trimmed.to_string(),
    );
    if let Some(path) = &document.path {
        add(
            "Extension",
            "Phần mở rộng",
            path.extension()
                .map(|e| e.to_string_lossy().into_owned())
                .unwrap_or_else(|| "—".into()),
        );
        match std::fs::symlink_metadata(path) {
            Ok(meta) => {
                add("Disk bytes", "Byte trên đĩa", meta.len().to_string());
                add("Created (UTC)", "Ngày tạo (UTC)", timestamp(meta.created()));
                add(
                    "Modified (UTC)",
                    "Sửa trên đĩa (UTC)",
                    timestamp(meta.modified()),
                );
                add(
                    "Accessed (UTC)",
                    "Truy cập (UTC)",
                    timestamp(meta.accessed()),
                );
                add(
                    "Disk read-only",
                    "Chỉ đọc trên đĩa",
                    if meta.permissions().readonly() {
                        yes
                    } else {
                        no
                    }
                    .into(),
                );
                if let Some(last) = &document.fingerprint {
                    let changed = last.size != meta.len() || last.modified != meta.modified().ok();
                    add(
                        "Disk metadata changed",
                        "Metadata đĩa đã đổi",
                        if changed { yes } else { no }.into(),
                    );
                }
            }
            Err(e) => add("Disk status", "Trạng thái đĩa", e.to_string()),
        }
    }
    if let Some(fingerprint) = &document.fingerprint {
        add(
            "Last verified SHA-256",
            "SHA-256 lần mở/lưu",
            fingerprint
                .sha256
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        );
    }
    Details {
        name: document
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| {
                if locale == Locale::Vi {
                    "Chưa đặt tên"
                } else {
                    "Untitled"
                }
                .into()
            }),
        language: document_language(document).label(),
        size: document.buffer().len_bytes(),
        lines: document.buffer().len_lines(),
        dirty: document.is_dirty(),
        fields,
    }
}
impl FileInfo {
    pub fn new(document: &Document, locale: Locale) -> Self {
        let history = document.history_status();
        let mut snapshot = document.worker_snapshot();
        snapshot.readonly = document.readonly;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(inspect(&snapshot, locale, history));
        });
        Self {
            details: None,
            top: 0,
            height: 10,
            error: None,
            pending: Some(rx),
        }
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn poll(&mut self) -> bool {
        let result = self.pending.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(details) => Some(Ok(details)),
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("File details worker stopped".into()))
            }
            Err(mpsc::TryRecvError::Empty) => None,
        });
        let Some(result) = result else {
            return false;
        };
        self.pending = None;
        match result {
            Ok(details) => self.details = Some(details),
            Err(error) => self.error = Some(error),
        }
        true
    }
    pub fn scroll(&mut self, delta: isize) {
        self.top = self.top.saturating_add_signed(delta).min(
            self.details
                .as_ref()
                .map_or(0, |d| d.fields.len().saturating_sub(1)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_buffer_and_disk_separately_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, "disk").unwrap();
        let mut doc = Document::from_text("e\u{301} 👩‍💻\r\n");
        doc.path = Some(path.clone());
        doc.insert("x").unwrap();
        let details = inspect(&doc, Locale::En, doc.history_status());
        assert!(details.dirty);
        assert_eq!(details.lines, 2);
        assert!(
            details
                .fields
                .iter()
                .any(|f| f.en == "Disk bytes" && f.value == "4")
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"disk");
    }
}
