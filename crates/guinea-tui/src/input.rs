//! Crossterm events on Unix; safe Win32 record adapter on Windows. Crossterm
//! 0.29's surrogate buffer consumes key-up records between UTF-16 halves under
//! ConPTY. Ignore releases BEFORE pairing, and respect native repeat counts.
use crossterm::event::Event;
use std::{io, time::Duration};

#[cfg(not(windows))]
pub struct Input;

#[cfg(not(windows))]
impl Input {
    pub fn new() -> Self {
        Self
    }
    pub fn waker(&self) -> Option<std::sync::Arc<dyn Fn() + Send + Sync>> {
        None
    }
    pub fn read(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
        if crossterm::event::poll(timeout)? {
            crossterm::event::read().map(Some)
        } else {
            Ok(None)
        }
    }
}

#[cfg(windows)]
pub struct Input {
    receiver: std::sync::mpsc::Receiver<io::Result<Event>>,
    sender: std::sync::mpsc::SyncSender<io::Result<Event>>,
    pending: Option<io::Result<Event>>,
}

#[cfg(windows)]
impl Input {
    pub fn new() -> Self {
        use crossterm::event::MouseEvent;
        use crossterm_winapi::{Console, Handle, InputRecord};
        let (sender, receiver) = std::sync::mpsc::sync_channel(1024);
        let wake_sender = sender.clone();
        std::thread::spawn(move || {
            let run = || -> io::Result<()> {
                let console = Console::from(Handle::current_in_handle()?);
                let mut decoder = WindowsKeys::default();
                let mut mouse_decoder = WindowsMouse::default();
                loop {
                    match console.read_single_input_event()? {
                        InputRecord::KeyEvent(record) => {
                            let flags = [0x01, 0x02, 0x04, 0x08, 0x10]
                                .into_iter()
                                .filter(|&bit| record.control_key_state.has_state(bit))
                                .sum();
                            if let Some(key) = decoder.decode(
                                record.u_char,
                                record.key_down,
                                record.virtual_key_code,
                                flags,
                            ) {
                                for _ in 0..record.repeat_count.max(1) {
                                    if sender.send(Ok(Event::Key(key))).is_err() {
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        InputRecord::WindowBufferSizeEvent(record) => {
                            sender
                                .send(Ok(Event::Resize(
                                    record.size.x.max(1) as u16,
                                    record.size.y.max(1) as u16,
                                )))
                                .map_err(|_| {
                                    io::Error::new(io::ErrorKind::BrokenPipe, "editor closed")
                                })?;
                        }
                        InputRecord::MouseEvent(record) => {
                            let flags = [0x01, 0x02, 0x04, 0x08, 0x10]
                                .into_iter()
                                .filter(|&bit| record.control_key_state.has_state(bit))
                                .sum();
                            for kind in mouse_decoder
                                .decode(record.button_state.state(), record.event_flags)
                            {
                                let event = Event::Mouse(MouseEvent {
                                    kind,
                                    column: record.mouse_position.x.max(0) as u16,
                                    row: record.mouse_position.y.max(0) as u16,
                                    modifiers: windows_modifiers(flags),
                                });
                                if sender.send(Ok(event)).is_err() {
                                    return Ok(());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            };
            if let Err(error) = run() {
                let _ = sender.send(Err(error));
            }
        });
        Self {
            receiver,
            sender: wake_sender,
            pending: None,
        }
    }
    pub fn waker(&self) -> Option<std::sync::Arc<dyn Fn() + Send + Sync>> {
        let sender = self.sender.clone();
        Some(std::sync::Arc::new(move || {
            let _ = sender.try_send(Ok(Event::FocusGained));
        }))
    }

    pub fn read(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
        use std::sync::mpsc::RecvTimeoutError;
        let received = if let Some(event) = self.pending.take() {
            Ok(event)
        } else {
            self.receiver.recv_timeout(timeout)
        };
        match received {
            Ok(Ok(Event::Key(key)))
                if key.code == crossterm::event::KeyCode::Enter && key.modifiers.is_empty() =>
            {
                // Legacy ConPTY records strip paste delimiters. Preserve an
                // adjacent CR/LF pair instead of inserting two policy newlines.
                match self.receiver.recv_timeout(Duration::from_millis(2)) {
                    Ok(Ok(Event::Key(next)))
                        if next.code == crossterm::event::KeyCode::Char('\n') =>
                    {
                        Ok(Some(Event::Paste("\r\n".into())))
                    }
                    Ok(next) => {
                        self.pending = Some(next);
                        Ok(Some(Event::Key(key)))
                    }
                    Err(_) => Ok(Some(Event::Key(key))),
                }
            }
            Ok(Ok(Event::Key(key))) if key.code == crossterm::event::KeyCode::Char('\n') => {
                Ok(Some(Event::Paste("\n".into())))
            }
            Ok(event) => event.map(Some),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => {
                Err(io::Error::other("console input reader stopped"))
            }
        }
    }
}

#[cfg(windows)]
fn windows_modifiers(flags: u32) -> crossterm::event::KeyModifiers {
    use crossterm::event::KeyModifiers;
    let mut modifiers = KeyModifiers::NONE;
    if flags & 0x10 != 0 {
        modifiers |= KeyModifiers::SHIFT;
    }
    if flags & 0x0c != 0 {
        modifiers |= KeyModifiers::CONTROL;
    }
    if flags & 0x03 != 0 {
        modifiers |= KeyModifiers::ALT;
    }
    modifiers
}

#[cfg(windows)]
#[derive(Default)]
struct WindowsMouse {
    pressed: [bool; 3],
}
#[cfg(windows)]
impl WindowsMouse {
    fn decode(
        &mut self,
        state: i32,
        flags: crossterm_winapi::EventFlags,
    ) -> Vec<crossterm::event::MouseEventKind> {
        use crossterm::event::{MouseButton, MouseEventKind};
        use crossterm_winapi::EventFlags;
        if flags == EventFlags::MouseWheeled {
            return vec![if state < 0 {
                MouseEventKind::ScrollDown
            } else {
                MouseEventKind::ScrollUp
            }];
        }
        if flags == EventFlags::MouseHwheeled {
            return vec![if state < 0 {
                MouseEventKind::ScrollLeft
            } else {
                MouseEventKind::ScrollRight
            }];
        }
        let buttons = [MouseButton::Left, MouseButton::Middle, MouseButton::Right];
        let pressed = [state & 1 != 0, state & 4 != 0, state & 2 != 0];
        let mut events = Vec::new();
        for (index, &button) in buttons.iter().enumerate() {
            if pressed[index] != self.pressed[index] {
                events.push(if pressed[index] {
                    MouseEventKind::Down(button)
                } else {
                    MouseEventKind::Up(button)
                });
            }
        }
        self.pressed = pressed;
        if flags == EventFlags::MouseMoved
            && let Some(index) = pressed.iter().position(|p| *p)
        {
            events.push(MouseEventKind::Drag(buttons[index]));
        }
        events
    }
}

#[cfg(windows)]
#[derive(Default)]
struct WindowsKeys {
    high_surrogate: Option<u16>,
}

#[cfg(windows)]
impl WindowsKeys {
    fn decode(
        &mut self,
        unit: u16,
        down: bool,
        virtual_key: u16,
        flags: u32,
    ) -> Option<crossterm::event::KeyEvent> {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        if !down {
            return None;
        }
        // ConPTY represents pasted LF as Ctrl+Enter. It is literal text,
        // not a Ctrl shortcut to be silently discarded by the editor.
        if unit == 10 && virtual_key == 0x0d {
            return Some(KeyEvent::new(KeyCode::Char('\n'), KeyModifiers::NONE));
        }
        let mut modifiers = KeyModifiers::NONE;
        if flags & 0x10 != 0 {
            modifiers |= KeyModifiers::SHIFT;
        }
        if flags & 0x0c != 0 {
            modifiers |= KeyModifiers::CONTROL;
        }
        if flags & 0x03 != 0 {
            modifiers |= KeyModifiers::ALT;
        }
        // AltGr produces text while reporting left Ctrl + right Alt.
        if flags & 0x09 == 0x09 && unit >= 0x20 {
            modifiers.remove(KeyModifiers::CONTROL | KeyModifiers::ALT);
        }
        if (0xd800..=0xdbff).contains(&unit) {
            self.high_surrogate = Some(unit);
            return None;
        }
        if (0xdc00..=0xdfff).contains(&unit) {
            let high = self.high_surrogate.take()?;
            let scalar = 0x10000 + ((u32::from(high) - 0xd800) << 10) + (u32::from(unit) - 0xdc00);
            return char::from_u32(scalar).map(|ch| KeyEvent::new(KeyCode::Char(ch), modifiers));
        }
        let code = match virtual_key {
            0x08 => KeyCode::Backspace,
            0x09 if modifiers.contains(KeyModifiers::SHIFT) => KeyCode::BackTab,
            0x09 => KeyCode::Tab,
            0x0d => KeyCode::Enter,
            0x1b => KeyCode::Esc,
            0x21 => KeyCode::PageUp,
            0x22 => KeyCode::PageDown,
            0x23 => KeyCode::End,
            0x24 => KeyCode::Home,
            0x25 => KeyCode::Left,
            0x26 => KeyCode::Up,
            0x27 => KeyCode::Right,
            0x28 => KeyCode::Down,
            0x2d => KeyCode::Insert,
            0x2e => KeyCode::Delete,
            vk @ 0x70..=0x87 => KeyCode::F((vk - 0x70 + 1) as u8),
            vk @ 0x41..=0x5a
                if (modifiers.contains(KeyModifiers::CONTROL)
                    && !modifiers.contains(KeyModifiers::ALT))
                    || (unit == 0
                        && modifiers.contains(KeyModifiers::ALT)
                        && !modifiers.contains(KeyModifiers::CONTROL)) =>
            {
                KeyCode::Char(
                    char::from_u32(u32::from(vk))
                        .expect("ASCII key")
                        .to_ascii_lowercase(),
                )
            }
            _ if unit >= 0x20 => KeyCode::Char(char::from_u32(u32::from(unit))?),
            _ => return None,
        };
        self.high_surrogate = None;
        Some(KeyEvent::new(code, modifiers))
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    #[test]
    fn conpty_linefeed_is_literal_text_not_an_ignored_ctrl_enter() {
        let mut keys = WindowsKeys::default();
        let key = keys.decode(10, true, 0x0d, 0x08).unwrap();
        assert_eq!(key.code, KeyCode::Char('\n'));
        assert!(key.modifiers.is_empty());
    }
    #[test]
    fn mouse_transitions_drag_buttons_horizontal_wheel_and_modifiers() {
        use crossterm::event::{MouseButton, MouseEventKind};
        use crossterm_winapi::EventFlags;
        let mut mouse = WindowsMouse::default();
        assert_eq!(
            mouse.decode(1, EventFlags::PressOrRelease),
            vec![MouseEventKind::Down(MouseButton::Left)]
        );
        assert_eq!(
            mouse.decode(1, EventFlags::MouseMoved),
            vec![MouseEventKind::Drag(MouseButton::Left)]
        );
        assert_eq!(
            mouse.decode(0, EventFlags::PressOrRelease),
            vec![MouseEventKind::Up(MouseButton::Left)]
        );
        for (bit, button) in [(4, MouseButton::Middle), (2, MouseButton::Right)] {
            assert_eq!(
                mouse.decode(bit, EventFlags::DoubleClick),
                vec![MouseEventKind::Down(button)]
            );
            assert_eq!(
                mouse.decode(0, EventFlags::PressOrRelease),
                vec![MouseEventKind::Up(button)]
            );
        }
        assert_eq!(
            mouse.decode(120 << 16, EventFlags::MouseHwheeled),
            vec![MouseEventKind::ScrollRight]
        );
        assert_eq!(
            mouse.decode(-120 << 16, EventFlags::MouseHwheeled),
            vec![MouseEventKind::ScrollLeft]
        );
        assert_eq!(
            windows_modifiers(0x18),
            KeyModifiers::SHIFT | KeyModifiers::CONTROL
        );
    }

    #[test]
    fn surrogate_keyup_between_halves_does_not_drop_emoji() {
        let mut decoder = WindowsKeys::default();
        for ch in "👩‍💻🇻🇳".chars() {
            let mut units = [0; 2];
            let encoded = ch.encode_utf16(&mut units);
            let mut events = Vec::new();
            for unit in encoded {
                if let Some(key) = decoder.decode(*unit, true, 0, 0) {
                    events.push(key);
                }
                assert!(decoder.decode(*unit, false, 0, 0).is_none());
            }
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].code, KeyCode::Char(ch));
        }
    }

    #[test]
    fn altgr_and_control_shortcuts_remain_distinct() {
        let mut decoder = WindowsKeys::default();
        assert_eq!(
            decoder.decode('@' as u16, true, 0, 0x09).unwrap().modifiers,
            KeyModifiers::NONE
        );
        let key = decoder.decode(19, true, 0x53, 0x08).unwrap();
        assert_eq!(key.code, KeyCode::Char('s'));
        assert_eq!(key.modifiers, KeyModifiers::CONTROL);
        let key = decoder.decode(0, true, 0x41, 0x02).unwrap();
        assert_eq!(key.code, KeyCode::Char('a'));
        assert_eq!(key.modifiers, KeyModifiers::ALT);
        let key = decoder.decode(0, true, 0x09, 0x18).unwrap();
        assert_eq!(key.code, KeyCode::BackTab);
        assert_eq!(crate::keyboard::tab_command(key), Some("prev"));
    }
}
