//! Keymap v1: the single source of truth for actions, their key bindings and
//! their F2 descriptions. The dispatcher, the F2 action list, the tab-switch
//! shortcuts inside dialogs and the documentation checks all read this table.
//!
//! Terminal policy (see docs/KEYBOARD_SHORTCUTS.md): every action has a key
//! that ordinary terminals transmit (F-keys, Ctrl+letter, Alt+letter or
//! Alt/Ctrl+arrows) and is always reachable from F2. Ctrl+Shift+letter,
//! Ctrl+Tab and similar combinations are listed only as extra aliases because
//! legacy terminal encodings cannot distinguish them or terminals reserve them.
use crate::i18n::Locale;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const NONE: KeyModifiers = KeyModifiers::NONE;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;
const ALT: KeyModifiers = KeyModifiers::ALT;
const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
const CTRL_SHIFT: KeyModifiers = KeyModifiers::CONTROL.union(KeyModifiers::SHIFT);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Main window without a dialog: editor, explorer or review.
    Global,
    /// Like Global, and also inside search/project dialogs and the tab
    /// manager (tab switching closes those dialogs safely).
    Tabs,
    /// Only while the source editor has focus (not explorer or full review).
    Editor,
}

pub struct Binding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
    pub label: &'static str,
}

const fn key(code: KeyCode, modifiers: KeyModifiers, label: &'static str) -> Binding {
    Binding {
        code,
        modifiers,
        label,
    }
}

pub struct Command {
    pub id: &'static str,
    pub en: &'static str,
    pub vi: &'static str,
    /// The first binding is the primary key shown in F2 and the guide.
    pub keys: &'static [Binding],
    pub scope: Scope,
}

macro_rules! command {
    ($id:literal, $en:literal, $vi:literal, $scope:ident, [$($key:expr),* $(,)?]) => {
        Command {
            id: $id,
            en: $en,
            vi: $vi,
            keys: &[$($key),*],
            scope: Scope::$scope,
        }
    };
}

use KeyCode::{Backspace, Char, Delete, Down, F, Left, PageDown, PageUp, Right, Tab, Up};

pub const COMMANDS: &[Command] = &[
    // F2 lists actions in this order; project search stays first.
    command!(
        "files",
        "Find a file in this project",
        "Tìm file trong project",
        Global,
        [key(Char('e'), CTRL, "Ctrl+E"),]
    ),
    command!(
        "project-search",
        "Find text across project files",
        "Tìm nội dung trong project",
        Global,
        [key(Char('g'), CTRL, "Ctrl+G"),]
    ),
    // Files and tabs.
    command!(
        "new",
        "New blank tab",
        "Tạo thẻ trống",
        Global,
        [
            key(Char('n'), CTRL, "Ctrl+N"),
            key(Char('t'), CTRL, "Ctrl+T"),
        ]
    ),
    command!(
        "open",
        "Open file or folder",
        "Mở file hoặc thư mục",
        Global,
        [key(Char('o'), CTRL, "Ctrl+O"),]
    ),
    command!(
        "open-folder",
        "Choose project folder",
        "Chọn thư mục project",
        Global,
        [key(Char('k'), CTRL, "Ctrl+K"),]
    ),
    command!(
        "save",
        "Save current file",
        "Lưu file hiện tại",
        Global,
        [key(Char('s'), CTRL, "Ctrl+S"),]
    ),
    command!(
        "save-as",
        "Save to a new path",
        "Lưu vào đường dẫn mới",
        Global,
        [
            key(Char('s'), ALT, "Alt+S"),
            key(Char('s'), CTRL_SHIFT, "Ctrl+Shift+S"),
        ]
    ),
    command!(
        "close",
        "Close tab",
        "Đóng thẻ",
        Global,
        [key(Char('w'), CTRL, "Ctrl+W")]
    ),
    command!(
        "quit",
        "Quit with unsaved-change protection",
        "Thoát, bảo vệ thay đổi chưa lưu",
        Global,
        [key(Char('q'), CTRL, "Ctrl+Q"),]
    ),
    command!(
        "tabs",
        "Manage all tabs",
        "Quản lý các thẻ",
        Tabs,
        [key(F(12), NONE, "F12"), key(Char('p'), CTRL, "Ctrl+P"),]
    ),
    command!(
        "prev",
        "Previous tab",
        "Thẻ trước",
        Tabs,
        [
            key(F(6), NONE, "F6"),
            key(F(6), SHIFT, "Shift+F6"),
            key(Left, ALT, "Alt+Left"),
            key(PageUp, CTRL, "Ctrl+PageUp"),
            key(Tab, CTRL_SHIFT, "Ctrl+Shift+Tab"),
        ]
    ),
    command!(
        "next",
        "Next tab",
        "Thẻ tiếp theo",
        Tabs,
        [
            key(F(7), NONE, "F7"),
            key(Right, ALT, "Alt+Right"),
            key(PageDown, CTRL, "Ctrl+PageDown"),
            key(Tab, CTRL, "Ctrl+Tab"),
        ]
    ),
    command!(
        "reload",
        "Reload from disk with confirmation",
        "Tải lại từ đĩa sau xác nhận",
        Global,
        []
    ),
    // Search and navigation.
    command!(
        "find",
        "Find and replace",
        "Tìm và thay thế",
        Global,
        [key(Char('f'), CTRL, "Ctrl+F")]
    ),
    command!(
        "replace",
        "Find and replace, replacement field",
        "Tìm và thay thế, ô thay bằng",
        Global,
        [key(Char('h'), CTRL, "Ctrl+H"),]
    ),
    command!(
        "find-next",
        "Find next match",
        "Tìm kết quả tiếp theo",
        Global,
        [key(F(3), NONE, "F3")]
    ),
    command!(
        "goto",
        "Go to line[:column]",
        "Đến dòng[:cột]",
        Global,
        [key(Char('l'), CTRL, "Ctrl+L")]
    ),
    // View.
    command!(
        "help",
        "Keyboard guide",
        "Hướng dẫn phím tắt",
        Global,
        [key(F(1), NONE, "F1")]
    ),
    command!(
        "commands",
        "Action list",
        "Danh sách thao tác",
        Global,
        [key(F(2), NONE, "F2")]
    ),
    command!(
        "focus",
        "Switch editor / directory tree",
        "Chuyển soạn thảo / cây thư mục",
        Global,
        [key(F(4), NONE, "F4"),]
    ),
    command!(
        "refresh",
        "Refresh directory tree",
        "Đọc lại cây thư mục",
        Global,
        [key(F(5), NONE, "F5")]
    ),
    command!(
        "preview",
        "Choose source/review layout",
        "Chọn bố cục xem trước",
        Global,
        [key(F(8), NONE, "F8"),]
    ),
    command!(
        "file-info",
        "File details",
        "Thông tin file",
        Global,
        [key(F(9), NONE, "F9")]
    ),
    command!(
        "settings",
        "Interface language",
        "Ngôn ngữ giao diện",
        Global,
        [key(F(10), NONE, "F10")]
    ),
    command!(
        "focus-mode",
        "Toggle distraction-free editor",
        "Bật hoặc tắt chế độ tập trung",
        Global,
        [key(F(11), NONE, "F11"), key(F(11), SHIFT, "Shift+F11"),]
    ),
    command!(
        "explorer",
        "Show or hide directory tree",
        "Hiện hoặc ẩn cây thư mục",
        Global,
        [key(Char('b'), CTRL, "Ctrl+B"),]
    ),
    command!(
        "outline",
        "Show or hide Markdown contents",
        "Hiện hoặc ẩn mục lục Markdown",
        Global,
        [key(Char('m'), ALT, "Alt+M"),]
    ),
    command!(
        "review-tab",
        "Open a linked read-only review tab",
        "Mở thẻ xem trước chỉ đọc",
        Global,
        []
    ),
    command!(
        "source",
        "Return to source",
        "Trở về nội dung nguồn",
        Global,
        []
    ),
    command!(
        "split",
        "Source and review side by side",
        "Nguồn và xem trước song song",
        Global,
        []
    ),
    // Editing.
    command!(
        "undo",
        "Undo",
        "Hoàn tác",
        Editor,
        [key(Char('z'), CTRL, "Ctrl+Z")]
    ),
    command!(
        "redo",
        "Redo",
        "Làm lại",
        Editor,
        [
            key(Char('y'), CTRL, "Ctrl+Y"),
            key(Char('z'), CTRL_SHIFT, "Ctrl+Shift+Z"),
        ]
    ),
    command!(
        "select-all",
        "Select whole document",
        "Chọn toàn bộ tài liệu",
        Editor,
        [key(Char('a'), CTRL, "Ctrl+A"),]
    ),
    command!(
        "copy",
        "Copy selection, or the line without a selection",
        "Sao chép vùng chọn, hoặc cả dòng khi không chọn",
        Editor,
        [key(Char('c'), CTRL, "Ctrl+C"),]
    ),
    command!(
        "cut",
        "Cut selection, or the line without a selection",
        "Cắt vùng chọn, hoặc cả dòng khi không chọn",
        Editor,
        [key(Char('x'), CTRL, "Ctrl+X"),]
    ),
    command!(
        "paste",
        "Paste from clipboard",
        "Dán từ bộ nhớ tạm",
        Editor,
        [key(Char('v'), CTRL, "Ctrl+V")]
    ),
    command!(
        "duplicate",
        "Duplicate line or selection",
        "Nhân đôi dòng hoặc vùng chọn",
        Editor,
        [key(Char('d'), CTRL, "Ctrl+D"),]
    ),
    command!(
        "move-line-up",
        "Move line(s) up",
        "Đưa dòng lên",
        Editor,
        [key(Up, ALT, "Alt+Up")]
    ),
    command!(
        "move-line-down",
        "Move line(s) down",
        "Đưa dòng xuống",
        Editor,
        [key(Down, ALT, "Alt+Down")]
    ),
    command!(
        "indent",
        "Indent selected lines",
        "Thụt lề các dòng đã chọn",
        Editor,
        [key(Tab, NONE, "Tab")]
    ),
    command!(
        "outdent",
        "Outdent selected lines",
        "Lùi lề các dòng đã chọn",
        Editor,
        [key(Tab, SHIFT, "Shift+Tab"),]
    ),
    command!(
        "word-left",
        "Previous word",
        "Từ trước",
        Editor,
        [key(Left, CTRL, "Ctrl+Left"), key(Char('b'), ALT, "Alt+B"),]
    ),
    command!(
        "word-right",
        "Next word",
        "Từ sau",
        Editor,
        [key(Right, CTRL, "Ctrl+Right"), key(Char('f'), ALT, "Alt+F"),]
    ),
    command!(
        "select-word-left",
        "Extend selection to previous word",
        "Mở rộng vùng chọn tới từ trước",
        Editor,
        [key(Left, CTRL_SHIFT, "Ctrl+Shift+Left"),]
    ),
    command!(
        "select-word-right",
        "Extend selection to next word",
        "Mở rộng vùng chọn tới từ sau",
        Editor,
        [key(Right, CTRL_SHIFT, "Ctrl+Shift+Right"),]
    ),
    command!(
        "delete-word-left",
        "Delete previous word",
        "Xóa từ phía trước",
        Editor,
        [
            key(Backspace, CTRL, "Ctrl+Backspace"),
            key(Backspace, ALT, "Alt+Backspace"),
        ]
    ),
    command!(
        "delete-word-right",
        "Delete next word",
        "Xóa từ phía sau",
        Editor,
        [
            key(Delete, CTRL, "Ctrl+Delete"),
            key(Char('d'), ALT, "Alt+D"),
        ]
    ),
];

/// Commands kept for compatibility with earlier F2 names; not listed.
pub fn canonical(id: &str) -> &str {
    match id {
        "info" => "file-info",
        "language" | "language-vi" | "language-en" | "open-path" => id,
        other => other,
    }
}

fn normalize(key: KeyEvent) -> (KeyCode, KeyModifiers) {
    let mut modifiers = key.modifiers & (CTRL | ALT | SHIFT);
    let code = match key.code {
        // Terminals report Shift+Tab as BackTab with or without SHIFT.
        KeyCode::BackTab => {
            modifiers |= SHIFT;
            Tab
        }
        // Letters may arrive upper-case with SHIFT; bindings use lower case.
        Char(c) if c.is_ascii_alphabetic() => Char(c.to_ascii_lowercase()),
        other => other,
    };
    (code, modifiers)
}

fn applies(scope: Scope, editor: bool, dialog: bool) -> bool {
    match scope {
        Scope::Tabs => true,
        Scope::Global => !dialog,
        Scope::Editor => editor && !dialog,
    }
}

/// The action bound to `key`. `editor` is true when the source editor has
/// focus; `dialog` is true inside tab-safe dialogs (only Tabs scope applies).
pub fn lookup(key: KeyEvent, editor: bool, dialog: bool) -> Option<&'static str> {
    let (code, modifiers) = normalize(key);
    COMMANDS
        .iter()
        .filter(|c| applies(c.scope, editor, dialog))
        .find(|c| {
            c.keys
                .iter()
                .any(|b| b.code == code && b.modifiers == modifiers)
        })
        .map(|c| c.id)
}

/// Tab switching/manager keys, valid in tab-safe dialogs too.
pub fn tab_command(key: KeyEvent) -> Option<&'static str> {
    lookup(key, false, true)
}

#[cfg(test)]
pub fn find(id: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.id == canonical(id))
}

pub fn keys(command: &Command) -> String {
    command
        .keys
        .iter()
        .map(|b| b.label)
        .collect::<Vec<_>>()
        .join(" / ")
}

pub fn suggestions(query: &str, locale: Locale) -> Vec<&'static Command> {
    let query = query.trim().to_lowercase();
    COMMANDS
        .iter()
        .filter(|c| c.id.contains(&query) || description(c, locale).to_lowercase().contains(&query))
        .collect()
}

pub fn description(command: &Command, locale: Locale) -> String {
    let text = if locale == Locale::Vi {
        command.vi
    } else {
        command.en
    };
    if command.keys.is_empty() {
        text.into()
    } else {
        format!("{text} · {}", keys(command))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keymap_has_unique_ids_and_no_conflicting_bindings() {
        let mut ids = std::collections::HashSet::new();
        for command in COMMANDS {
            assert!(ids.insert(command.id), "duplicate id {}", command.id);
        }
        for (i, a) in COMMANDS.iter().enumerate() {
            for b in &COMMANDS[i + 1..] {
                for x in a.keys {
                    for y in b.keys {
                        assert!(
                            !(x.code == y.code && x.modifiers == y.modifiers),
                            "{} and {} both bind {}",
                            a.id,
                            b.id,
                            x.label
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_bound_action_has_a_terminal_portable_primary_key() {
        for command in COMMANDS.iter().filter(|c| !c.keys.is_empty()) {
            let primary = &command.keys[0];
            // Ctrl+Shift+letter is indistinguishable from Ctrl+letter in
            // legacy terminal encodings, so it may only be an alias.
            assert!(
                !(primary.modifiers.contains(CTRL | SHIFT) && matches!(primary.code, Char(_))),
                "{} primary key {} is not portable",
                command.id,
                primary.label
            );
            // Ctrl+M, Ctrl+I, Ctrl+[ and Ctrl+J are Enter/Tab/Esc/LF.
            assert!(
                !(primary.modifiers == CTRL && matches!(primary.code, Char('m' | 'i' | '[' | 'j'))),
                "{} uses a control code that terminals send for another key",
                command.id
            );
        }
    }

    #[test]
    fn lookup_normalizes_case_backtab_and_respects_scope() {
        let upper = KeyEvent::new(Char('S'), CTRL);
        assert_eq!(lookup(upper, true, false), Some("save"));
        let alt_s = KeyEvent::new(Char('S'), ALT | SHIFT);
        assert_eq!(lookup(alt_s, true, false), None, "Alt+Shift+S is not Alt+S");
        let backtab = KeyEvent::new(KeyCode::BackTab, NONE);
        assert_eq!(lookup(backtab, true, false), Some("outdent"));
        assert_eq!(
            lookup(backtab, false, false),
            None,
            "explorer keeps Shift+Tab"
        );
        let undo = KeyEvent::new(Char('z'), CTRL);
        assert_eq!(lookup(undo, false, false), None);
        assert_eq!(lookup(KeyEvent::new(F(6), NONE), false, true), Some("prev"));
        assert_eq!(lookup(KeyEvent::new(F(1), NONE), false, true), None);
        assert_eq!(find("info").unwrap().id, "file-info");
        assert!(description(find("save").unwrap(), Locale::Vi).ends_with("· Ctrl+S"));
    }

    #[test]
    fn keyboard_guides_mention_every_primary_key() {
        for command in COMMANDS.iter().filter(|c| !c.keys.is_empty()) {
            let label = command.keys[0].label;
            for (name, guide) in [
                ("EN", crate::keyboard::GUIDE_EN),
                ("VI", crate::keyboard::GUIDE_VI),
            ] {
                assert!(
                    guide.contains(label),
                    "{name} guide does not mention {label} ({})",
                    command.id
                );
            }
        }
    }
}
