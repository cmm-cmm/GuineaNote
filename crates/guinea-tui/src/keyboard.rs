//! Portable tab navigation and a keyboard guide readable at every supported size.
use crossterm::event::KeyEvent;
#[cfg(test)]
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::text::Line;
use unicode_segmentation::UnicodeSegmentation;

/// Tab switching keys, valid in tab-safe dialogs; defined by the keymap.
pub fn tab_command(key: KeyEvent) -> Option<&'static str> {
    crate::commands::tab_command(key)
}

pub const GUIDE_EN: &str = "Guinea Note — keyboard guide

F1 Help · Esc close a dialog · F2 action list
F2: arrows choose, Enter runs, typing filters

TABS
Ctrl+P / F12 manage all tabs
F6 previous / F7 next tab
Alt+Right / Alt+Left next / previous tab
Ctrl+Tab / Ctrl+Shift+Tab when delivered by terminal
Ctrl+PageDown / Ctrl+PageUp next / previous tab
Ctrl+N / Ctrl+T new · Ctrl+W close
Manager: type to filter · Ctrl+U clear
Manager: arrows / Tab / Shift+Tab choose
Manager: Enter switch · Ctrl+S save · Ctrl+W close

FIND / REPLACE — Ctrl+F or Ctrl+H
One panel, both fields always available
Ctrl+F query · Ctrl+H replacement · Ctrl+U clear field
Tab / Shift+Tab fields, results and buttons
Enter activates a focused button; results Enter returns
Enter / F3 next · Shift+Enter / Shift+F3 previous
Ctrl+R replace one · Alt+A replace all
Alt+C match case on/off · Alt+W whole word on/off
Ctrl+Shift+R also replaces all where supported
Esc close · Ctrl+Z undo in editor · Ctrl+S save

FILES AND EDITING
Ctrl+O open (file picker) · Ctrl+S save
Alt+S / Ctrl+Shift+S save as · Ctrl+L go to line
Ctrl+Z / Ctrl+Y undo / redo · Ctrl+A select all
Ctrl+C / Ctrl+X / Ctrl+V copy / cut / paste (system clipboard)
Arrows move · Shift selects · Ctrl+Home/End start/end
Ctrl+Q quit; dirty tabs require confirmation

EDITING
Ctrl+Left / Ctrl+Right previous / next word (also Alt+B / Alt+F)
Ctrl+Shift+Left / Ctrl+Shift+Right select by word
Ctrl+Backspace / Alt+Backspace delete previous word
Ctrl+Delete / Alt+D delete next word
Tab / Shift+Tab indent / outdent selected lines
Enter keeps the line's indentation
Ctrl+D duplicate line or selection
Alt+Up / Alt+Down move line(s) up / down
Ctrl+C / Ctrl+X without a selection: whole line
Typing is one Undo step until a pause or caret move
F2 line tools: sort-lines, remove-duplicate-lines, remove-empty-lines,
  trim-trailing, upper-case, lower-case, title-case, toggle-comment

VIEW
File picker: arrows choose · Tab complete · Alt+Up parent
F11 / Shift+F11 focus mode · F10 settings · F9 file details
F8 split/review chooser: arrows or Tab, Enter apply
Review: arrows / PgUp/PgDn / Home/End · Esc source
Alt+M Markdown contents · Tab focus outline

DIALOGS
Settings: arrows / Tab / Shift+Tab choose · Enter apply
Confirm: Tab / arrows choose · Enter act · Esc cancel
Y confirms discard · N cancels; cancel is the default
Input: Ctrl+U clear · Enter submit · Esc cancel
Help: arrows / PgUp/PgDn scroll · Home/End jump

WHEN THE TERMINAL KEEPS A KEY
F2 runs every action by name
Windows Terminal F11 fullscreen: use Shift+F11
Ctrl+Tab / Ctrl+PageUp switch terminal tabs: use F6 / F7
GNU screen prefix Ctrl+A: F2 select-all
Ctrl+Backspace sent as Ctrl+H on Unix: use Alt+Backspace

UTF-8 only · resident memory · periodic draft recovery";

pub const GUIDE_VI: &str = "Guinea Note — hướng dẫn phím tắt

F1 Trợ giúp · Esc đóng hộp thoại · F2 danh sách thao tác
F2: mũi tên chọn, Enter chạy, gõ để lọc

QUẢN LÝ THẺ
Ctrl+P / F12 quản lý toàn bộ thẻ
F6 thẻ trước / F7 thẻ sau
Alt+Right / Alt+Left thẻ sau / thẻ trước
Ctrl+Tab / Ctrl+Shift+Tab khi terminal truyền được
Ctrl+PageDown / Ctrl+PageUp thẻ sau / thẻ trước
Ctrl+N / Ctrl+T tạo thẻ · Ctrl+W đóng thẻ
Danh sách: gõ để lọc · Ctrl+U xóa lọc
Danh sách: mũi tên / Tab / Shift+Tab chọn
Danh sách: Enter chuyển · Ctrl+S lưu · Ctrl+W đóng

TÌM / THAY THẾ — Ctrl+F hoặc Ctrl+H
Một bảng chung, luôn có đủ hai ô nhập
Ctrl+F ô tìm · Ctrl+H ô thay · Ctrl+U xóa ô
Tab / Shift+Tab qua ô nhập, kết quả và các nút
Enter kích hoạt nút; Enter ở kết quả về soạn thảo
Enter / F3 sau · Shift+Enter / Shift+F3 trước
Ctrl+R thay một · Alt+A thay tất cả
Alt+C bật/tắt phân biệt hoa thường · Alt+W bật/tắt khớp cả từ
Ctrl+Shift+R cũng thay tất cả nếu terminal hỗ trợ
Esc đóng · Ctrl+Z hoàn tác ở editor · Ctrl+S lưu

FILE VÀ SOẠN THẢO
Ctrl+O mở (chọn file) · Ctrl+S lưu
Alt+S / Ctrl+Shift+S lưu thành · Ctrl+L đến dòng
Ctrl+Z / Ctrl+Y hoàn tác / làm lại · Ctrl+A chọn hết
Ctrl+C / Ctrl+X / Ctrl+V sao chép / cắt / dán (bộ nhớ tạm hệ thống)
Mũi tên di chuyển · Shift chọn · Ctrl+Home/End đầu/cuối
Ctrl+Q thoát; thay đổi chưa lưu cần xác nhận

SOẠN THẢO
Ctrl+Left / Ctrl+Right từ trước / sau (hoặc Alt+B / Alt+F)
Ctrl+Shift+Left / Ctrl+Shift+Right chọn theo từ
Ctrl+Backspace / Alt+Backspace xóa từ phía trước
Ctrl+Delete / Alt+D xóa từ phía sau
Tab / Shift+Tab thụt / lùi lề các dòng đã chọn
Enter giữ thụt lề của dòng
Ctrl+D nhân đôi dòng hoặc vùng chọn
Alt+Up / Alt+Down đưa dòng lên / xuống
Ctrl+C / Ctrl+X khi không chọn: cả dòng
Gõ liên tục là một bước hoàn tác tới khi dừng hoặc di chuyển
Công cụ dòng trong F2: sort-lines, remove-duplicate-lines, remove-empty-lines,
  trim-trailing, upper-case, lower-case, title-case, toggle-comment

HIỂN THỊ
Chọn file: mũi tên chọn · Tab điền · Alt+Up thư mục cha
F11 / Shift+F11 tập trung · F10 thiết lập · F9 thông tin file
F8 chọn xem: mũi tên hoặc Tab, Enter áp dụng
Xem trước: mũi tên / PgUp/PgDn / Home/End · Esc nguồn
Alt+M mục lục Markdown · Tab chuyển vùng mục lục

HỘP THOẠI
Thiết lập: mũi tên / Tab / Shift+Tab chọn · Enter áp dụng
Xác nhận: Tab / mũi tên chọn · Enter chạy · Esc hủy
Y xác nhận bỏ · N hủy; mặc định luôn là hủy
Ô nhập: Ctrl+U xóa · Enter gửi · Esc hủy
Trợ giúp: mũi tên / PgUp/PgDn cuộn · Home/End đầu/cuối

KHI TERMINAL GIỮ PHÍM
F2 chạy mọi thao tác theo tên
Windows Terminal dùng F11 toàn màn hình: dùng Shift+F11
Ctrl+Tab / Ctrl+PageUp đổi thẻ terminal: dùng F6 / F7
GNU screen dùng Ctrl+A: F2 select-all
Unix gửi Ctrl+Backspace thành Ctrl+H: dùng Alt+Backspace

Chỉ UTF-8 · dữ liệu trong RAM · khôi phục bản nháp";

pub fn help_lines(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for source in text.lines() {
        let mut line = String::new();
        let mut cells = 0;
        for grapheme in source.graphemes(true) {
            let count = Line::from(grapheme).width();
            if cells + count > width && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                cells = 0;
            }
            line.push_str(grapheme);
            cells += count;
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reverse_tab_and_alternative_keys_do_not_depend_on_shift_encoding() {
        assert_eq!(
            tab_command(KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE)),
            Some("prev")
        );
        assert_eq!(
            tab_command(KeyEvent::new(KeyCode::F(7), KeyModifiers::NONE)),
            Some("next")
        );
        for key in [
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::CONTROL),
            KeyEvent::new(KeyCode::Tab, KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            KeyEvent::new(KeyCode::Left, KeyModifiers::ALT),
            KeyEvent::new(KeyCode::F(6), KeyModifiers::SHIFT),
        ] {
            assert_eq!(tab_command(key), Some("prev"));
        }
        assert_eq!(
            tab_command(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            tab_command(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            tab_command(KeyEvent::new(
                KeyCode::Char('p'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )),
            None
        );
    }

    #[test]
    fn help_wraps_without_losing_vietnamese_or_splitting_graphemes() {
        let text = "e\u{301} 👩‍💻 界 Việt ".repeat(8);
        for width in [22, 30, 76] {
            let lines = help_lines(&text, width);
            assert_eq!(lines.concat(), text);
            assert!(
                lines
                    .iter()
                    .all(|line| Line::from(line.as_str()).width() <= width)
            );
            assert!(
                lines
                    .iter()
                    .all(|line| !line.starts_with('\u{301}') && !line.starts_with('\u{200d}'))
            );
        }
    }
}
