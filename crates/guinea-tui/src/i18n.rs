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
            " Files ^E " => " File ^E ",
            " Project ^G " => " Project ^G ",
            "Go to line[:column] (1-based)" => "Đến dòng[:cột] (bắt đầu từ 1)",
            "Invalid line or column" => "Dòng hoặc cột không hợp lệ",
            "Indexing long line… keep typing or moving" => {
                "Đang lập chỉ mục dòng dài… vẫn có thể gõ hoặc di chuyển"
            }
            "Indexing long line… click again when it finishes" => {
                "Đang lập chỉ mục dòng dài… hãy nhấp lại khi hoàn tất"
            }
            "Search result changed; current buffer retained. Find again in this tab." => {
                "Kết quả tìm đã thay đổi; giữ nội dung hiện tại. Hãy tìm lại trong thẻ này."
            }
            "Project match opened" => "Đã mở vị trí kết quả trong project",
            " F11 Workspace   ^S Save   ^E Files   ^G Project search   ^P Tabs   F2 Commands" => {
                " F11 Bố cục   ^S Lưu   ^E File   ^G Tìm project   ^P Thẻ   F2 Lệnh"
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

            "EXPLORER" => "THƯ MỤC",
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
            " ↑↓ Navigate   ^C Copy   F6/F7 Other tabs   ^T New tab   F10 Settings" => {
                " ↑↓ Di chuyển   ^C Sao chép   F6/F7 Thẻ khác   ^T Thẻ mới   F10 Thiết lập"
            }
            "REVIEW · F8" => "XEM TRƯỚC · F8",
            "Welcome" => "Chào bạn",
            "Untitled" => "Chưa đặt tên",
            " AI · READ ONLY " => " AI · CHỈ ĐỌC ",
            "AI is working in this tab; read-only until it finishes" => {
                "AI đang thao tác thẻ này; chỉ đọc cho đến khi hoàn tất"
            }
            "Tabs" => "Thẻ",
            "unsaved" => "chưa lưu",
            " + New " => " + Mới ",
            " Save " => " Lưu ",
            " Close " => " Đóng ",
            " All ^P " => " Tất cả ^P ",
            " All ^P F12 " => " Thẻ ^P F12 ",
            " + New ^T " => " + Mới ^T ",
            " Close ^W " => " Đóng ^W ",
            " ^S Save   ^F Find/Replace   F6 Prev   F7 Next   ^P Tabs   F1 Help   F4 Pane   F8 Review   ^Q Quit" => {
                " ^S Lưu   ^F Tìm/Thay   F6 Trước   F7 Sau   ^P Thẻ   F1 Trợ giúp   F4 Khung   F8 Xem   ^Q Thoát"
            }
            "    A little more focused." => "    Tập trung từng ý tưởng.",
            "WORKSPACE  /  v0.1.0 " => "KHÔNG GIAN  /  v0.1.0 ",
            " Settings F10 " => " Thiết lập F10 ",
            " Info F9 " => " Chi tiết F9 ",
            "Settings saved" => "Đã lưu thiết lập",
            "Saving settings…" => "Đang lưu thiết lập…",
            "Saving settings… Esc cancels exit" => "Đang lưu thiết lập… Esc hủy thoát",
            "Settings applied for this session" => "Đã áp dụng thiết lập cho phiên này",
            "Review tab is read-only; return to source to edit or save" => {
                "Tab review chỉ đọc; về tab nguồn để chỉnh sửa hoặc lưu"
            }
            "Loading files…" => "Đang đọc danh sách file…",
            "Opening path…" => "Đang mở đường dẫn…",
            " Open file  ^O " => " Mở file  ^O ",
            " Open folder  ^K " => " Mở thư mục  ^K ",
            " New  ^N " => " Tạo mới  ^N ",
            " Save  ^S " => " Lưu  ^S ",
            " Find ^F " => " Tìm ^F ",
            " Find/Replace ^F " => " Tìm/Thay ^F ",
            " Replace ^H " => " Thay ^H ",
            " Open ^O " => " Mở ^O ",
            " Folder ^K " => " Thư mục ^K ",
            " New ^N " => " Mới ^N ",
            " Save ^S " => " Lưu ^S ",
            " View F8 " => " Xem F8 ",
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
            " Review  F8 " => " Xem trước  F8 ",
            "F1 Help   F2 Commands " => "F1 Trợ giúp  F2 Lệnh ",
            "  Loading files…" => "  Đang đọc thư mục…",
            "  Cannot read folder" => "  Không đọc được thư mục",
            "  This folder is empty" => "  Thư mục trống",
            " Loading…" => " Đang tải…",
            " Entry cap reached · open subfolder" => " Đạt giới hạn · mở thư mục con",
            "Your files. Your flow. Your terminal." => "File của bạn. Nhịp làm việc của bạn.",
            "Open a file" => "Mở một file",
            "Open a folder" => "Mở một thư mục",
            "Start a new document" => "Tạo tài liệu mới",
            "F2 Commands     F1 Keyboard guide" => "F2 Danh sách lệnh    F1 Phím tắt",
            "Welcome / your next idea starts here" => "Chào bạn / ý tưởng tiếp theo bắt đầu ở đây",
            " READ ONLY " => " CHỈ ĐỌC ",
            " FILES " => " THƯ MỤC ",
            " EDIT " => " SOẠN ",
            " REVIEW " => " XEM ",
            "  ● MODIFIED" => "  ● ĐÃ SỬA",
            "  New buffer" => "  File mới",
            "  ● Saved" => "  ● Đã lưu",
            "KEYBOARD GUIDE" => "HƯỚNG DẪN PHÍM TẮT",
            "UNSAVED CHANGES / CONFIRM" => "THAY ĐỔI CHƯA LƯU / XÁC NHẬN",
            "COMMANDS" => "DANH SÁCH LỆNH",
            "OPEN / SEARCH / SAVE" => "MỞ / TÌM / LƯU",
            "Open file or folder" => "Mở file hoặc thư mục",
            "Open folder" => "Mở thư mục",
            "Save to NEW path" => "Lưu vào đường dẫn MỚI",
            "Find literal (case sensitive)" => "Tìm chính xác (phân biệt hoa thường)",
            "Replace all: find literal" => "Thay tất cả: nội dung cần tìm",
            "Replace all: replacement (Enter allows empty)" => {
                "Thay tất cả: nội dung mới (có thể để trống)"
            }
            "Command (F1 lists commands)" => "Nhập lệnh (F1 xem danh sách)",
            "Ready — open a file or a folder to begin" => {
                "Sẵn sàng — mở file hoặc thư mục để bắt đầu"
            }
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
            "That path is not an accessible folder" => {
                "Đường dẫn không phải thư mục có thể truy cập"
            }
            "Empty input; cancelled" => "Nội dung trống; đã hủy",
            "Untitled buffer has no file to reload" => {
                "File chưa đặt tên chưa có dữ liệu trên đĩa để tải lại"
            }
            "Workspace opened — Enter opens a file · F4 switches panes" => {
                "Đã mở thư mục — Enter mở file · F4 chuyển khung"
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
            "Explorer needs a wider terminal; Ctrl+O still opens files" => {
                "Cần terminal rộng hơn để hiện cây thư mục; Ctrl+O vẫn mở được file"
            }
            " ↑↓ Browse   → Expand   ← Collapse   Enter Open   Tab Editor   ^B Hide files" => {
                " ↑↓ Duyệt   → Mở nhánh   ← Thu gọn   Enter Mở   Tab Soạn thảo   ^B Ẩn cây"
            }
            " ^S Save   ^O Open   ^K Folder   ^B Files   F4 Pane   F6 Tabs   F8 Review   F2 Commands   ^Q Quit" => {
                " ^S Lưu   ^O Mở   ^K Thư mục   ^B Cây   F4 Khung   F6 Tab   F8 Xem   F2 Lệnh   ^Q Thoát"
            }
            "GUINEA NOTE\nResize to at least 30 × 9\nCtrl+Q to quit" => {
                "GUINEA NOTE\nCần tối thiểu 30 × 9 ô\nCtrl+Q để thoát"
            }
            "GUINEA NOTE\n\nCtrl+O  Open file\nCtrl+K  Open folder\nCtrl+N  New document" => {
                "GUINEA NOTE\n\nCtrl+O  Mở file\nCtrl+K  Mở thư mục\nCtrl+N  Tài liệu mới"
            }
            "\n\nEnter accepts     Esc cancels" => "\n\nEnter chấp nhận     Esc hủy",
            "\n\nnew  open  open-folder  save  save-as  close\nexplorer  focus  refresh  find  replace  quit\n\nType a command · Enter runs · Esc closes" => {
                "\n\nnew  open  open-folder  save  save-as  close\nexplorer  focus  refresh  find  replace  quit\npreview  review  source  split  language-vi  language-en\n\nNhập lệnh · Enter thực hiện · Esc đóng"
            }
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

    pub fn message(self, text: &str) -> String {
        let translated = self.tr(text);
        if translated != text || self == Self::En {
            return translated.into_owned();
        }
        if let Some(tail) = text
            .strip_prefix("Replaced ")
            .and_then(|s| s.strip_suffix(" occurrence(s); Ctrl+Z undoes the transaction"))
        {
            return format!("Đã thay {tail} kết quả; Ctrl+Z hoàn tác toàn bộ");
        }
        if let Some(tail) = text.strip_prefix("Cannot read ") {
            return format!("Không đọc được {tail}");
        }
        if let Some(tail) =
            text.strip_suffix(". Resident RAM may be much larger. Y open / N cancel")
            && let Some((size, limit)) = tail.split_once(" bytes exceeds warning ")
        {
            return format!(
                "{size} byte vượt ngưỡng cảnh báo {limit}. RAM sử dụng có thể lớn hơn. Y mở / N hủy"
            );
        }
        for (prefix, vi) in [
            (
                "Settings apply to this session; save failed: ",
                "Đã áp dụng cho phiên này; lưu thiết lập thất bại: ",
            ),
            ("Opening ", "Đang mở "),
            (
                "System clipboard unavailable; copied internally: ",
                "Không dùng được bộ nhớ tạm hệ thống; đã sao chép nội bộ: ",
            ),
            (
                "System clipboard unavailable; pasted internal clipboard: ",
                "Không dùng được bộ nhớ tạm hệ thống; đã dán bộ nhớ nội bộ: ",
            ),
            ("Saving ", "Đang lưu "),
            ("Unknown command: ", "Lệnh không tồn tại: "),
        ] {
            if let Some(tail) = text.strip_prefix(prefix) {
                let tail = tail
                    .strip_suffix("… edits paused")
                    .map(|s| format!("{s}… tạm dừng sửa"))
                    .or_else(|| {
                        tail.strip_suffix("… you can keep editing")
                            .map(|s| format!("{s}… bạn có thể tiếp tục sửa"))
                    })
                    .unwrap_or_else(|| tail.to_string());
                return format!("{vi}{tail}");
            }
        }
        text.to_string()
    }
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
            Locale::Vi.message("Saving D:/Saved/EDITOR.md… you can keep editing"),
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
}
