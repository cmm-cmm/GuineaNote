//! Clipboard integration. The internal clipboard always holds the last copy,
//! so copy/paste keep working when the system clipboard is unavailable.
//!
//! Modes (TOML `clipboard`, or `GUINEANOTE_CLIPBOARD` for harnesses):
//! - `auto` (default): system clipboard locally, internal over SSH.
//! - `system`: Windows clipboard API; `pbcopy`/`pbpaste` on macOS; `wl-copy`/
//!   `wl-paste`, `xclip` or `xsel` elsewhere. Runs on a worker thread.
//! - `osc52`: copy through the terminal with an OSC 52 escape sequence (useful
//!   over SSH); paste uses the internal clipboard or the terminal's own paste.
//! - `internal`: never touch the system clipboard.
use serde::Deserialize;
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// Many terminals cap OSC 52 payloads around 100 KB of base64.
pub const OSC52_LIMIT: usize = 74 * 1024;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Auto,
    Internal,
    System,
    Osc52,
}

impl Mode {
    pub fn from_env() -> Option<Self> {
        match std::env::var("GUINEANOTE_CLIPBOARD").ok()?.as_str() {
            "auto" => Some(Self::Auto),
            "internal" => Some(Self::Internal),
            "system" => Some(Self::System),
            "osc52" => Some(Self::Osc52),
            _ => None,
        }
    }
    fn resolve(self) -> Self {
        match self {
            // Unit tests must never replace the developer's real clipboard.
            Self::Auto if cfg!(test) => Self::Internal,
            Self::Auto
                if std::env::var_os("SSH_CONNECTION").is_some()
                    || std::env::var_os("SSH_TTY").is_some() =>
            {
                Self::Internal
            }
            Self::Auto => Self::System,
            other => other,
        }
    }
}

pub enum Copied {
    /// Sent to the system clipboard on a worker; failures arrive via `poll`.
    System,
    Osc52,
    Internal,
    /// Larger than [`OSC52_LIMIT`]; kept in the internal clipboard only.
    TooLargeForOsc52,
}

pub enum Paste {
    Ready(String),
    /// The system clipboard is read on a worker; the text arrives via `poll`.
    Pending,
}

pub enum Event {
    Pasted(String),
    /// System read failed; the internal clipboard text is supplied instead.
    PasteFallback(String, String),
    CopyFailed(String),
}

pub struct Clipboard {
    mode: Mode,
    internal: String,
    pasting: Option<Receiver<Result<String, String>>>,
    copying: Option<Receiver<Result<(), String>>>,
    /// OSC 52 sequence for the event loop to write after the next frame.
    pub osc52: Option<String>,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new(Mode::Internal)
    }
}

impl Clipboard {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode: mode.resolve(),
            internal: String::new(),
            pasting: None,
            copying: None,
            osc52: None,
        }
    }
    #[cfg(test)]
    pub fn mode(&self) -> Mode {
        self.mode
    }
    pub fn busy(&self) -> bool {
        self.pasting.is_some() || self.copying.is_some()
    }

    pub fn copy(&mut self, text: String) -> Copied {
        match self.mode {
            Mode::System => {
                let (sender, receiver) = mpsc::channel();
                let copy = text.clone();
                std::thread::spawn(move || {
                    let _ = sender.send(system::set(&copy));
                });
                self.copying = Some(receiver);
                self.internal = text;
                Copied::System
            }
            Mode::Osc52 if text.len() > OSC52_LIMIT => {
                self.internal = text;
                Copied::TooLargeForOsc52
            }
            Mode::Osc52 => {
                self.osc52 = Some(osc52(&text));
                self.internal = text;
                Copied::Osc52
            }
            Mode::Auto | Mode::Internal => {
                self.internal = text;
                Copied::Internal
            }
        }
    }

    pub fn paste(&mut self) -> Paste {
        if self.mode != Mode::System {
            return Paste::Ready(self.internal.clone());
        }
        if self.pasting.is_none() {
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let _ = sender.send(system::get());
            });
            self.pasting = Some(receiver);
        }
        Paste::Pending
    }

    pub fn poll(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        if let Some(receiver) = &self.copying {
            match receiver.try_recv() {
                Ok(Ok(())) => self.copying = None,
                Ok(Err(error)) => {
                    self.copying = None;
                    events.push(Event::CopyFailed(error));
                }
                Err(TryRecvError::Disconnected) => {
                    self.copying = None;
                    events.push(Event::CopyFailed("clipboard worker stopped".into()));
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(receiver) = &self.pasting {
            let result = match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Disconnected) => Some(Err("clipboard worker stopped".into())),
                Err(TryRecvError::Empty) => None,
            };
            if let Some(result) = result {
                self.pasting = None;
                events.push(match result {
                    Ok(text) => Event::Pasted(text),
                    Err(error) => Event::PasteFallback(self.internal.clone(), error),
                });
            }
        }
        events
    }
}

/// `ESC ] 52 ; c ; <base64> BEL`, the widely supported OSC 52 copy form.
pub fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(TABLE[(n >> shift) as usize & 63] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(windows)]
mod system {
    pub fn set(text: &str) -> Result<(), String> {
        clipboard_win::set_clipboard_string(text).map_err(|e| e.to_string())
    }
    pub fn get() -> Result<String, String> {
        clipboard_win::get_clipboard_string().map_err(|e| e.to_string())
    }
}

#[cfg(not(windows))]
mod system {
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn candidates(read: bool) -> Vec<(&'static str, &'static [&'static str])> {
        if cfg!(target_os = "macos") {
            return vec![if read {
                ("pbpaste", &[])
            } else {
                ("pbcopy", &[])
            }];
        }
        let mut list = Vec::new();
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            list.push(if read {
                ("wl-paste", &["--no-newline"][..])
            } else {
                ("wl-copy", &[][..])
            });
        }
        list.push(if read {
            ("xclip", &["-selection", "clipboard", "-o"][..])
        } else {
            ("xclip", &["-selection", "clipboard"][..])
        });
        list.push(if read {
            ("xsel", &["--clipboard", "--output"][..])
        } else {
            ("xsel", &["--clipboard", "--input"][..])
        });
        list
    }

    pub fn set(text: &str) -> Result<(), String> {
        let mut last = String::from("no clipboard tool found (pbcopy, wl-copy, xclip, xsel)");
        for (program, args) in candidates(false) {
            let child = Command::new(program)
                .args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            let Ok(mut child) = child else {
                continue;
            };
            // The pipe is closed at the end of this statement, so the tool sees EOF.
            if let Some(mut stdin) = child.stdin.take()
                && let Err(error) = stdin.write_all(text.as_bytes())
            {
                last = error.to_string();
                let _ = child.wait();
                continue;
            }
            match child.wait() {
                Ok(status) if status.success() => return Ok(()),
                Ok(status) => last = format!("{program} exited with {status}"),
                Err(error) => last = error.to_string(),
            }
        }
        Err(last)
    }

    pub fn get() -> Result<String, String> {
        let mut last = String::from("no clipboard tool found (pbpaste, wl-paste, xclip, xsel)");
        for (program, args) in candidates(true) {
            match Command::new(program)
                .args(args)
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output()
            {
                Ok(output) if output.status.success() => {
                    return String::from_utf8(output.stdout)
                        .map_err(|_| "clipboard text is not UTF-8".to_string());
                }
                Ok(output) => last = format!("{program} exited with {}", output.status),
                Err(_) => {}
            }
        }
        Err(last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_and_osc52_encode_unicode_exactly() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64("Việt".as_bytes()), "Vmnhu4d0");
        assert_eq!(osc52("hi"), "\x1b]52;c;aGk=\x07");
    }

    #[test]
    fn auto_mode_never_touches_the_system_clipboard_in_tests() {
        let mut clipboard = Clipboard::new(Mode::Auto);
        assert_eq!(clipboard.mode(), Mode::Internal);
        assert!(matches!(clipboard.copy("Việt".into()), Copied::Internal));
        assert!(matches!(clipboard.paste(), Paste::Ready(text) if text == "Việt"));
        assert!(!clipboard.busy());
    }

    #[test]
    fn osc52_copies_through_the_terminal_and_caps_large_selections() {
        let mut clipboard = Clipboard::new(Mode::Osc52);
        assert!(matches!(clipboard.copy("e\u{301}".into()), Copied::Osc52));
        assert_eq!(clipboard.osc52.take().unwrap(), osc52("e\u{301}"));
        let large = "x".repeat(OSC52_LIMIT + 1);
        assert!(matches!(
            clipboard.copy(large.clone()),
            Copied::TooLargeForOsc52
        ));
        assert!(clipboard.osc52.is_none());
        // The internal clipboard still holds the selection for Ctrl+V.
        assert!(matches!(clipboard.paste(), Paste::Ready(text) if text == large));
    }
}
