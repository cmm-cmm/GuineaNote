//! Discoverable F2 actions. Stable command ids keep terminal fallbacks portable.
use crate::i18n::Locale;
pub struct Command {
    pub id: &'static str,
    pub en: &'static str,
    pub vi: &'static str,
}
pub const COMMANDS: &[Command] = &[
    Command {
        id: "files",
        en: "Find a file in this project · Ctrl+E",
        vi: "Tìm file trong project · Ctrl+E",
    },
    Command {
        id: "project-search",
        en: "Find text across project files · Ctrl+G",
        vi: "Tìm nội dung trong project · Ctrl+G",
    },
    Command {
        id: "open",
        en: "Open file or folder · Ctrl+O",
        vi: "Mở file hoặc thư mục · Ctrl+O",
    },
    Command {
        id: "new",
        en: "New blank tab · Ctrl+N",
        vi: "Tạo thẻ trống · Ctrl+N",
    },
    Command {
        id: "save",
        en: "Save current file · Ctrl+S",
        vi: "Lưu file hiện tại · Ctrl+S",
    },
    Command {
        id: "save-as",
        en: "Save to a new path · Alt+S",
        vi: "Lưu vào đường dẫn mới · Alt+S",
    },
    Command {
        id: "goto",
        en: "Go to line[:column] · Ctrl+L",
        vi: "Đến dòng[:cột] · Ctrl+L",
    },
    Command {
        id: "find",
        en: "Find and replace · Ctrl+F / Ctrl+H",
        vi: "Tìm và thay thế · Ctrl+F / Ctrl+H",
    },
    Command {
        id: "tabs",
        en: "Manage all tabs · Ctrl+P / F12",
        vi: "Quản lý các thẻ · Ctrl+P / F12",
    },
    Command {
        id: "open-folder",
        en: "Choose project folder · Ctrl+K",
        vi: "Chọn thư mục project · Ctrl+K",
    },
    Command {
        id: "explorer",
        en: "Show or hide directory tree · Ctrl+B",
        vi: "Hiện hoặc ẩn cây thư mục · Ctrl+B",
    },
    Command {
        id: "focus",
        en: "Switch editor / directory tree · F4",
        vi: "Chuyển soạn thảo / cây thư mục · F4",
    },
    Command {
        id: "focus-mode",
        en: "Toggle distraction-free editor · F11",
        vi: "Bật hoặc tắt chế độ tập trung · F11",
    },
    Command {
        id: "review-tab",
        en: "Open a linked read-only review tab",
        vi: "Mở thẻ xem trước chỉ đọc",
    },
    Command {
        id: "preview",
        en: "Choose source/review layout · F8",
        vi: "Chọn bố cục xem trước · F8",
    },
    Command {
        id: "source",
        en: "Return to source",
        vi: "Trở về nội dung nguồn",
    },
    Command {
        id: "split",
        en: "Source and review side by side",
        vi: "Nguồn và xem trước song song",
    },
    Command {
        id: "settings",
        en: "Interface language · F10",
        vi: "Ngôn ngữ giao diện · F10",
    },
    Command {
        id: "file-info",
        en: "File details · F9",
        vi: "Thông tin file · F9",
    },
    Command {
        id: "undo",
        en: "Undo · Ctrl+Z",
        vi: "Hoàn tác · Ctrl+Z",
    },
    Command {
        id: "redo",
        en: "Redo · Ctrl+Y",
        vi: "Làm lại · Ctrl+Y",
    },
    Command {
        id: "copy",
        en: "Copy selection · Ctrl+C",
        vi: "Sao chép phần chọn · Ctrl+C",
    },
    Command {
        id: "paste",
        en: "Paste from clipboard · Ctrl+V",
        vi: "Dán từ bộ nhớ tạm · Ctrl+V",
    },
    Command {
        id: "cut",
        en: "Cut selection · Ctrl+X",
        vi: "Cắt phần chọn · Ctrl+X",
    },
    Command {
        id: "select-all",
        en: "Select whole document · Ctrl+A",
        vi: "Chọn toàn bộ tài liệu · Ctrl+A",
    },
    Command {
        id: "next",
        en: "Next tab · F7 / Alt+Right",
        vi: "Thẻ tiếp theo · F7 / Alt+Right",
    },
    Command {
        id: "prev",
        en: "Previous tab · F6 / Alt+Left",
        vi: "Thẻ trước · F6 / Alt+Left",
    },
    Command {
        id: "refresh",
        en: "Refresh directory tree · F5",
        vi: "Đọc lại cây thư mục · F5",
    },
    Command {
        id: "reload",
        en: "Reload from disk with confirmation",
        vi: "Tải lại từ đĩa sau xác nhận",
    },
    Command {
        id: "close",
        en: "Close tab · Ctrl+W",
        vi: "Đóng thẻ · Ctrl+W",
    },
    Command {
        id: "quit",
        en: "Quit with unsaved-change protection · Ctrl+Q",
        vi: "Thoát, bảo vệ thay đổi chưa lưu · Ctrl+Q",
    },
    Command {
        id: "help",
        en: "Keyboard guide · F1",
        vi: "Hướng dẫn phím tắt · F1",
    },
];
pub fn suggestions(query: &str, locale: Locale) -> Vec<&'static Command> {
    let query = query.trim().to_lowercase();
    COMMANDS
        .iter()
        .filter(|c| c.id.contains(&query) || description(c, locale).to_lowercase().contains(&query))
        .collect()
}
pub fn description(command: &Command, locale: Locale) -> &'static str {
    if locale == Locale::Vi {
        command.vi
    } else {
        command.en
    }
}
