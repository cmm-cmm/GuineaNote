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
}

#[cfg(windows)]
impl Input {
    pub fn new() -> Self {
        use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
        use crossterm_winapi::{Console, EventFlags, Handle, InputRecord};
        let (sender, receiver) = std::sync::mpsc::sync_channel(1024);
        let wake_sender = sender.clone();
        std::thread::spawn(move || {
            let run = || -> io::Result<()> {
                let console = Console::from(Handle::current_in_handle()?);
                let mut decoder = WindowsKeys::default();
                let mut left_down = false;
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
                            let pressed = record.button_state.left_button();
                            let kind = match record.event_flags {
                                EventFlags::MouseWheeled if record.button_state.scroll_down() => {
                                    Some(MouseEventKind::ScrollDown)
                                }
                                EventFlags::MouseWheeled => Some(MouseEventKind::ScrollUp),
                                EventFlags::PressOrRelease | EventFlags::DoubleClick
                                    if pressed && !left_down =>
                                {
                                    Some(MouseEventKind::Down(MouseButton::Left))
                                }
                                EventFlags::PressOrRelease if !pressed && left_down => {
                                    Some(MouseEventKind::Up(MouseButton::Left))
                                }
                                _ => None,
                            };
                            left_down = pressed;
                            if let Some(kind) = kind {
                                let event = Event::Mouse(MouseEvent {
                                    kind,
                                    column: record.mouse_position.x.max(0) as u16,
                                    row: record.mouse_position.y.max(0) as u16,
                                    modifiers: KeyModifiers::NONE,
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
        match self.receiver.recv_timeout(timeout) {
            Ok(event) => event.map(Some),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => {
                Err(io::Error::other("console input reader stopped"))
            }
        }
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
                if modifiers.contains(KeyModifiers::CONTROL)
                    && !modifiers.contains(KeyModifiers::ALT) =>
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
    }
}
