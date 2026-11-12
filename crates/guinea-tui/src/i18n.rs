//! Only interface strings are translated. Document text and paths never pass
//! through this catalog. Command names remain stable in both locales.
use serde::Deserialize;
use std::borrow::Cow;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    Vi,
    #[default]
    En,
}

impl Locale {
    pub fn ai_action(self, action: &str) -> Cow<'static, str> {
        self.tr(match action {
            "read" => "Reading",
            "search" => "Searching",
            "analyze" => "Analyzing",
            "plan patch" => "Planning changes",
            "apply patch" => "Applying changes",
            "save" => "Saving",
            "review" => "Opening review",
            _ => "Working",
        })
    }
    pub fn toggle(self) -> Self {
        if self == Self::Vi { Self::En } else { Self::Vi }
    }
    pub fn tr<'a>(self, text: &'a str) -> Cow<'a, str> {
        if self == Self::En {
            return Cow::Borrowed(text);
        }
        let translated = match text {
            "Go to line[:column] (1-based)" => "Đến dòng[:cột] (bắt đầu từ 1)",
            "Invalid line or column" => "Dòng hoặc cột không hợp lệ",
            "Indexing long line… keep typing or moving" => {
                "Đang lập chỉ mục dòng dài… vẫn có thể gõ hoặc di chuyển"
            }
            "Indexing long line… click again when it finishes" => {
                "Đang lập chỉ mục dòng dài… hãy nhấp lại khi hoàn tất"
            }
            "Recovered" => "Khôi phục",
            "Recovered drafts opened; use Save As to keep them" => {
                "Đã mở bản nháp khôi phục; dùng Lưu thành để giữ lại"
            }
            "Recovery warning" => "Cảnh báo khôi phục bản nháp",
            "Older Undo steps trimmed to keep history within its budget" => {
                "Đã bỏ các bước Hoàn tác cũ vượt giới hạn lịch sử"
            }
            "… Review limited to 100,000 rows or 16 MiB of display budget; source is complete." => {
                "… Xem trước giới hạn 100.000 dòng hoặc 16 MiB ngân sách hiển thị; nguồn vẫn đầy đủ."
            }

            "EDITOR" => "SOẠN THẢO",
            "START HERE" => "BẮT ĐẦU",
            "SOURCE" => "NGUỒN",
            "SPLIT" => "SONG SONG",
            "REVIEW" => "XEM TRƯỚC",
            "Reading" => "Đang đọc",
            "Searching" => "Đang tìm",
            "Analyzing" => "Đang phân tích",
            "Planning changes" => "Lập thay đổi",
            "Applying changes" => "Áp dụng thay đổi",
            "Saving" => "Đang lưu",
            "Opening review" => "Đang mở xem trước",
            "Working" => "Đang xử lý",
            "AI released this tab; original access restored" => {
                "AI đã trả quyền thẻ; khôi phục trạng thái truy cập ban đầu"
            }
            "REVIEW · F8" => "XEM TRƯỚC · F8",
            "Welcome" => "Chào bạn",
            "Untitled" => "Chưa đặt tên",
            " AI · READ ONLY " => " AI · CHỈ ĐỌC ",
            "AI is working in this tab; read-only until it finishes" => {
                "AI đang thao tác thẻ này; chỉ đọc cho đến khi hoàn tất"
            }
            "Tabs" => "Thẻ",
            "Replacing" => "Đang thay thế",
            "Finding" => "Đang tìm",
            "Open file" => "Mở file",
            "Open" => "Mở",
            "Find/Replace" => "Tìm/Thay",
            "Find" => "Tìm",
            "Review" => "Xem trước",
            "View" => "Xem",
            "Settings" => "Thiết lập",
            "Info" => "Chi tiết",
            "New" => "Mới",
            "Save" => "Lưu",
            "Close" => "Đóng",
            "All" => "Tất cả thẻ",
            "Help" => "Trợ giúp",
            "Commands" => "Lệnh",
            "Exit focus" => "Thoát tập trung",
            "Navigate" => "Di chuyển",
            "Copy" => "Sao chép",
            "Prev" => "Trước",
            "Next" => "Sau",
            "New tab" => "Thẻ mới",
            "Quit" => "Thoát",
            "Keyboard guide" => "Hướng dẫn phím",
            "Type a file path" => "Nhập đường dẫn file",
            "Nothing to change" => "Không có gì để thay đổi",
            "Moved to line" => "Đã đến dòng",
            "This file type has no line comment" => "Loại file này không có comment dòng",
            "GUINEA NOTE\n\nCtrl+O  Open file\nCtrl+N  New document" => {
                "GUINEA NOTE\n\nCtrl+O  Mở file\nCtrl+N  Tài liệu mới"
            }
            "Ready — open a file to begin" => "Sẵn sàng — mở một file để bắt đầu",
            "unsaved" => "chưa lưu",
            " + New " => " + Mới ",
            " Save " => " Lưu ",
            " Close " => " Đóng ",
            "    A little more focused." => "    Tập trung từng ý tưởng.",
            "WORKSPACE  /  v0.1.0 " => "KHÔNG GIAN  /  v0.1.0 ",
            "Settings saved" => "Đã lưu thiết lập",
            "Saving settings…" => "Đang lưu thiết lập…",
            "Saving settings… Esc cancels exit" => "Đang lưu thiết lập… Esc hủy thoát",
            "Settings applied for this session" => "Đã áp dụng thiết lập cho phiên này",
            "Review tab is read-only; return to source to edit or save" => {
                "Tab review chỉ đọc; về tab nguồn để chỉnh sửa hoặc lưu"
            }
            "Loading files…" => "Đang đọc danh sách file…",
            "Opening path…" => "Đang mở đường dẫn…",
            "Search closed" => "Đã đóng tìm kiếm",
            "Input is limited to 4096 UTF-8 bytes" => "Ô nhập giới hạn 4096 byte UTF-8",
            "Document changed; search again before replacing" => {
                "Tài liệu đã thay đổi; tìm lại trước khi thay thế"
            }
            "Document is no longer active" => "Tài liệu không còn được chọn",
            "Search worker stopped; edit the query to retry" => {
                "Tìm kiếm bị dừng; sửa từ khóa để thử lại"
            }
            "Search cancelled" => "Đã hủy tìm kiếm",
            "Result exceeds 128 MiB; use a shorter replacement or replace one match" => {
                "Kết quả vượt 128 MiB; giảm nội dung thay thế hoặc thay từng mục"
            }
            "Your files. Your flow. Your terminal." => "File của bạn. Nhịp làm việc của bạn.",
            "Open a file" => "Mở một file",
            "Start a new document" => "Tạo tài liệu mới",
            "Welcome / your next idea starts here" => "Chào bạn / ý tưởng tiếp theo bắt đầu ở đây",
            " READ ONLY " => " CHỈ ĐỌC ",
            " EDIT " => " SOẠN ",
            " REVIEW " => " XEM ",
            "  ● MODIFIED" => "  ● ĐÃ SỬA",
            "  New buffer" => "  File mới",
            "  ● Saved" => "  ● Đã lưu",
            "KEYBOARD GUIDE" => "HƯỚNG DẪN PHÍM TẮT",
            "UNSAVED CHANGES / CONFIRM" => "THAY ĐỔI CHƯA LƯU / XÁC NHẬN",
            "COMMANDS" => "DANH SÁCH LỆNH",
            "OPEN / SEARCH / SAVE" => "MỞ / TÌM / LƯU",
            "File path to open" => "Đường dẫn file cần mở",
            "Save to NEW path" => "Lưu vào đường dẫn MỚI",
            "Find literal (case sensitive)" => "Tìm chính xác (phân biệt hoa thường)",
            "Replace all: find literal" => "Thay tất cả: nội dung cần tìm",
            "Replace all: replacement (Enter allows empty)" => {
                "Thay tất cả: nội dung mới (có thể để trống)"
            }
            "Command (F1 lists commands)" => "Nhập lệnh (F1 xem danh sách)",
            "Already open; switched to its tab" => "File đã mở; đã chuyển sang tab tương ứng",
            "Review is read-only; F8 or Esc returns to source" => {
                "Bản xem chỉ đọc; F8 hoặc Esc để về nguồn"
            }
            "Finding… edits paused" => "Đang tìm… tạm dừng sửa",
            "Replacing… edits paused" => "Đang thay thế… tạm dừng sửa",
            "unsupported encoding: only strict UTF-8 (optionally BOM) can be edited" => {
                "Mã hóa chưa được hỗ trợ: chỉ sửa được UTF-8 hợp lệ (có thể có BOM)"
            }
            "NUL/binary input is not supported; original file was not modified" => {
                "Chưa hỗ trợ dữ liệu nhị phân/NUL; file gốc được giữ nguyên"
            }
            "file changed outside GuineaNote; reload, save to a new path, or cancel" => {
                "File đã thay đổi bên ngoài GuineaNote; tải lại, lưu vào đường dẫn mới hoặc hủy"
            }
            "document or destination is read-only" => "Tài liệu hoặc nơi lưu chỉ cho phép đọc",
            "symbolic links and non-regular files are refused; open the real file path" => {
                "Không mở liên kết tượng trưng hoặc file đặc biệt; hãy mở đường dẫn file thật"
            }
            "destination already exists; open it in a tab before editing instead of overwriting" => {
                "Đường dẫn đích đã tồn tại; hãy mở file đó trong một tab để chỉnh sửa"
            }
            "file changed while it was being read; retry opening" => {
                "File thay đổi trong khi đọc; hãy thử mở lại"
            }
            "Opened" => "Đã mở",
            "Saved" => "Đã lưu",
            "Reloaded" => "Đã tải lại",
            "Cancelled" => "Đã hủy",
            "Document is read-only" | "document is read-only" => "Tài liệu chỉ cho phép đọc",
            "Copied to internal clipboard" => "Đã sao chép vào bộ nhớ tạm của ứng dụng",
            "Copied to clipboard" => "Đã sao chép vào bộ nhớ tạm",
            "Copied through the terminal (OSC 52)" => "Đã sao chép qua terminal (OSC 52)",
            "Selection too large for OSC 52; copied to internal clipboard" => {
                "Vùng chọn quá lớn cho OSC 52; đã sao chép vào bộ nhớ tạm của ứng dụng"
            }
            "Pasting from system clipboard…" => "Đang dán từ bộ nhớ tạm hệ thống…",
            "Paste cancelled: the active tab changed" => "Đã hủy dán: thẻ đang mở đã thay đổi",
            "Clipboard is empty" => "Bộ nhớ tạm đang trống",
            "Empty input; cancelled" => "Nội dung trống; đã hủy",
            "Untitled buffer has no file to reload" => {
                "File chưa đặt tên chưa có dữ liệu trên đĩa để tải lại"
            }
            "Background operation in progress; edits paused until it completes" => {
                "Đang xử lý nền; tạm dừng sửa cho tới khi hoàn tất"
            }
            "Background operation in progress; retry when complete" => {
                "Đang xử lý nền; hãy thử lại khi hoàn tất"
            }
            "File worker failed; document retained. Retry or save to a new path." => {
                "Tác vụ file gặp lỗi; nội dung được giữ lại. Thử lại hoặc lưu vào đường dẫn mới."
            }
            "Discard unsaved changes in this tab? Y discard / N cancel (Ctrl+S saves after cancel)" => {
                "Bỏ thay đổi chưa lưu trong tab này? Y bỏ / N hủy (Ctrl+S để lưu sau khi hủy)"
            }
            "Discard ALL unsaved tabs and quit? Y discard / N cancel (save tabs after cancel)" => {
                "Bỏ TẤT CẢ thay đổi chưa lưu và thoát? Y bỏ / N hủy (lưu các tab sau khi hủy)"
            }
            "Reload from disk and discard this tab's edits/history? Y reload / N cancel" => {
                "Tải lại từ đĩa và bỏ thay đổi/lịch sử của tab này? Y tải lại / N hủy"
            }
            "Match selected (F3 next)" => "Đã chọn kết quả (F3 tìm tiếp)",
            "No grapheme-aligned match" => "Không tìm thấy kết quả khớp trọn cụm ký tự",
            "F8 Source / Split / Review · review never changes the source" => {
                "F8 Nguồn / Chia đôi / Xem trước · chế độ xem giữ nguyên dữ liệu nguồn"
            }
            "Review supports Markdown, JSON/JSONL, XML, CSV/TSV, TOML, YAML and INI" => {
                "Xem trước hỗ trợ Markdown, JSON/JSONL, XML, CSV/TSV, TOML, YAML và INI"
            }
            "Rendering review… source remains editable in split view" => {
                "Đang dựng bản xem… có thể sửa nguồn ở chế độ chia đôi"
            }
            "Invalid data · fix the source to refresh review" => {
                "Dữ liệu chưa hợp lệ · sửa nguồn để cập nhật bản xem"
            }
            "Updating review…" => "Đang cập nhật bản xem…",
            "↑↓ Scroll · F8 View · Esc Source" => "↑↓ Cuộn · F8 Chế độ xem · Esc Về nguồn",
            "Review worker failed; source retained" => {
                "Tác vụ xem trước gặp lỗi; nguồn được giữ nguyên"
            }
            "GUINEA NOTE\nResize to at least 30 × 9\nCtrl+Q to quit" => {
                "GUINEA NOTE\nCần tối thiểu 30 × 9 ô\nCtrl+Q để thoát"
            }
            "\n\nEnter accepts     Esc cancels" => "\n\nEnter chấp nhận     Esc hủy",
            "[Y] Confirm     [N / Esc] Keep working" => {
                "[Y] Xác nhận     [N / Esc] Tiếp tục làm việc"
            }
            "JSON · formatted view\n\n" => "JSON · bản xem có định dạng\n\n",
            "XML · structure view\n\n" => "XML · bản xem cấu trúc\n\n",
            "TOML · formatted view\n\n" => "TOML · bản xem có định dạng\n\n",
            "… Review limited to 100,000 display rows; source is complete." => {
                "… Bản xem giới hạn 100.000 dòng hiển thị; nguồn vẫn đầy đủ."
            }
            "[DOCTYPE retained in source; external entities are not loaded]" => {
                "[DOCTYPE được giữ trong nguồn; không tải thực thể bên ngoài]"
            }
            _ => return Cow::Borrowed(text),
        };
        Cow::Borrowed(translated)
    }

    /// Static status text is keyed by its English literal (see `tr`).
    /// Parameterized messages are built with [`Locale::render`] when they are
    /// set, so nothing is reverse-parsed here. The only remaining prefix rule
    /// covers OS read errors reported by components that do not know the
    /// locale (file picker and explorer statuses).
    pub fn message(self, text: &str) -> String {
        let translated = self.tr(text);
        if translated != text || self == Self::En {
            return translated.into_owned();
        }
        if let Some(tail) = text.strip_prefix("Cannot read ") {
            return format!("Không đọc được {tail}");
        }
        text.to_string()
    }

    /// Localized parameterized status messages.
    pub fn render(self, message: Msg<'_>) -> String {
        let vi = self == Self::Vi;
        match message {
            Msg::Opening(path) if vi => format!("Đang mở {}… tạm dừng sửa", path.display()),
            Msg::Opening(path) => format!("Opening {}… edits paused", path.display()),
            Msg::Saving(path) if vi => {
                format!("Đang lưu {}… bạn có thể tiếp tục sửa", path.display())
            }
            Msg::Saving(path) => format!("Saving {}… you can keep editing", path.display()),
            Msg::Working(label) if vi => format!("{}… tạm dừng sửa", self.tr(label)),
            Msg::Working(label) => format!("{label}… edits paused"),
            Msg::SettingsSaveFailed(error) if vi => {
                format!("Đã áp dụng cho phiên này; lưu thiết lập thất bại: {error}")
            }
            Msg::SettingsSaveFailed(error) => {
                format!("Settings apply to this session; save failed: {error}")
            }
            Msg::UnknownCommand(command) if vi => {
                format!("Lệnh không tồn tại: {command}. F1 liệt kê các lệnh.")
            }
            Msg::UnknownCommand(command) => {
                format!("Unknown command: {command}. F1 lists commands.")
            }
            Msg::LargeFile { size, limit } if vi => format!(
                "{size} byte vượt ngưỡng cảnh báo {limit}. RAM sử dụng có thể lớn hơn. Y mở / N hủy"
            ),
            Msg::LargeFile { size, limit } => format!(
                "{size} bytes exceeds warning {limit}. Resident RAM may be much larger. Y open / N cancel"
            ),
            Msg::CopyFailed(error) if vi => {
                format!("Không dùng được bộ nhớ tạm hệ thống; đã sao chép nội bộ: {error}")
            }
            Msg::CopyFailed(error) => {
                format!("System clipboard unavailable; copied internally: {error}")
            }
            Msg::PasteFallback(error) if vi => {
                format!("Không dùng được bộ nhớ tạm hệ thống; đã dán bộ nhớ nội bộ: {error}")
            }
            Msg::PasteFallback(error) => {
                format!("System clipboard unavailable; pasted internal clipboard: {error}")
            }
            Msg::AgentSession { session, write } => format!(
                "{}: {session} · {}",
                if vi { "Phiên Agent" } else { "Agent session" },
                match (vi, write) {
                    (true, true) => "được ghi",
                    (true, false) => "chỉ đọc",
                    (false, true) => "write",
                    (false, false) => "read-only",
                }
            ),
        }
    }
}

/// Status messages with parameters; see [`Locale::render`].
pub enum Msg<'a> {
    Opening(&'a std::path::Path),
    Saving(&'a std::path::Path),
    /// A background computation; the label is a static English key.
    Working(&'a str),
    SettingsSaveFailed(&'a str),
    UnknownCommand(&'a str),
    LargeFile {
        size: u64,
        limit: u64,
    },
    CopyFailed(&'a str),
    PasteFallback(&'a str),
    AgentSession {
        session: &'a str,
        write: bool,
    },
}

pub const HELP_VI: &str = crate::keyboard::GUIDE_VI;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translates_ui_and_keeps_paths_intact() {
        assert_eq!(Locale::Vi.tr("EDITOR"), "SOẠN THẢO");
        assert_eq!(Locale::En.tr("EDITOR"), "EDITOR");
        assert_eq!(
            Locale::Vi.render(Msg::Saving(std::path::Path::new("D:/Saved/EDITOR.md"))),
            "Đang lưu D:/Saved/EDITOR.md… bạn có thể tiếp tục sửa"
        );
        assert!(toml::from_str::<crate::config::Config>("language = 'fr'").is_err());
        assert_eq!(
            toml::from_str::<crate::config::Config>("language = 'vi'")
                .unwrap()
                .language,
            Locale::Vi
        );
    }

    #[test]
    fn every_status_literal_has_a_vietnamese_translation() {
        // Status text is keyed by its English literal. A literal set as a
        // status (or returned as an unavailable-action reason) without a
        // catalog entry would silently show English in Vietnamese mode.
        let sources = [
            include_str!("app.rs"),
            include_str!("command_app.rs"),
            include_str!("modal_app.rs"),
            include_str!("mouse_app.rs"),
            include_str!("search_app.rs"),
            include_str!("agent.rs"),
            include_str!("workspace_render.rs"),
        ];
        let mut missing = Vec::new();
        for source in sources {
            let code = source.split("#[cfg(test)]").next().unwrap_or(source);
            for line in code.lines() {
                if !(line.contains("message = ")
                    || line.contains("Some(\"")
                    || line.contains("Copied::"))
                {
                    continue;
                }
                for (index, part) in line.split('"').enumerate() {
                    let literal = index % 2 == 1
                        && part.chars().next().is_some_and(char::is_uppercase)
                        && part.contains(' ');
                    if literal && Locale::Vi.tr(part) == part {
                        missing.push(part.to_string());
                    }
                }
            }
        }
        assert!(missing.is_empty(), "untranslated status text: {missing:#?}");
    }
}
