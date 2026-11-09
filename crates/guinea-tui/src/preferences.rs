use crate::i18n::Locale;
use serde::Deserialize;
use std::{
    io::{self, Write},
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
};

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Stored {
    language: Locale,
}

pub fn path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("GUINEANOTE_PREFERENCES") {
        return Some(path.into());
    }
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("GuineaNote/preferences.toml"))
    } else if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        Some(PathBuf::from(path).join("guineanote/preferences.toml"))
    } else {
        std::env::var_os("HOME")
            .map(|p| PathBuf::from(p).join(".config/guineanote/preferences.toml"))
    }
}

pub fn load() -> Result<Option<Locale>, Box<dyn std::error::Error>> {
    let Some(path) = path() else {
        return Ok(None);
    };
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(toml::from_str::<Stored>(&text)?.language)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn save(path: &std::path::Path, language: Locale) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    writeln!(
        file,
        "language = \"{}\"",
        if language == Locale::Vi { "vi" } else { "en" }
    )?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[derive(Default)]
pub struct Preferences {
    pub path: Option<PathBuf>,
    pending: Option<Receiver<Result<(), String>>>,
    queued: Option<Locale>,
}
impl Preferences {
    pub fn busy(&self) -> bool {
        self.pending.is_some() || self.queued.is_some()
    }
    pub fn request(&mut self, language: Locale) {
        if self.path.is_some() {
            self.queued = Some(language);
        }
    }
    pub fn poll(&mut self) -> Option<Result<(), String>> {
        let completed = self.pending.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Settings worker stopped".into())),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if completed.is_some() {
            self.pending = None;
        }
        if self.pending.is_none()
            && let Some(language) = self.queued.take()
            && let Some(path) = self.path.clone()
        {
            let (tx, rx) = mpsc::channel();
            self.pending = Some(rx);
            thread::spawn(move || {
                let _ = tx.send(save(&path, language).map_err(|e| e.to_string()));
            });
        }
        completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rapid_changes_keep_last_selected_language() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("preferences.toml");
        let mut state = Preferences {
            path: Some(path.clone()),
            ..Preferences::default()
        };
        state.request(Locale::Vi);
        state.poll();
        state.request(Locale::En);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while state.busy() {
            state.poll();
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(
            toml::from_str::<Stored>(&std::fs::read_to_string(path).unwrap())
                .unwrap()
                .language,
            Locale::En
        );
    }
    #[test]
    fn persists_only_language_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prefs/settings.toml");
        save(&path, Locale::Vi).unwrap();
        assert_eq!(
            toml::from_str::<Stored>(&std::fs::read_to_string(&path).unwrap())
                .unwrap()
                .language,
            Locale::Vi
        );
        save(&path, Locale::En).unwrap();
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1
        );
    }
}
