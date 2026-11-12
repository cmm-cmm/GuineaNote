use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub language: crate::i18n::Locale,
    pub tab_width: usize,
    pub large_file_warning_bytes: u64,
    pub syntax: bool,
    pub sidebar_width: u16,
    pub basic_colors: bool,
    pub mouse: bool,
    pub recovery: bool,
    pub no_color: bool,
    pub ansi_colors: bool,
    /// `auto`, `system`, `osc52` or `internal`; see `clipboard.rs`.
    pub clipboard: crate::clipboard::Mode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: crate::i18n::Locale::En,
            tab_width: 4,
            large_file_warning_bytes: 10 * 1024 * 1024,
            syntax: true,
            sidebar_width: 30,
            basic_colors: !cfg!(windows)
                && !std::env::var("COLORTERM")
                    .is_ok_and(|value| matches!(value.as_str(), "truecolor" | "24bit")),
            mouse: true,
            recovery: true,
            no_color: std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty()),
            ansi_colors: std::env::var("TERM")
                .is_ok_and(|term| matches!(term.as_str(), "linux" | "vt100" | "vt102" | "ansi")),
            clipboard: crate::clipboard::Mode::from_env().unwrap_or_default(),
        }
    }
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let config: Self = match path {
            Some(path) => {
                let value: toml::Value = toml::from_str(&fs::read_to_string(path)?)?;
                let explicit_language = value.get("language").is_some();
                let mut config: Self = value.try_into()?;
                if !explicit_language && let Some(language) = crate::preferences::load()? {
                    config.language = language;
                }
                config
            }
            None => {
                let mut config = Self::default();
                if let Some(language) = crate::preferences::load()? {
                    config.language = language;
                }
                config
            }
        };
        if !(1..=16).contains(&config.tab_width) {
            return Err("tab_width must be between 1 and 16".into());
        }
        if config.large_file_warning_bytes == 0 {
            return Err("large_file_warning_bytes must be positive".into());
        }
        if !(22..=50).contains(&config.sidebar_width) {
            return Err("sidebar_width must be between 22 and 50".into());
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_has_safe_defaults_and_rejects_unknown_fields() {
        let config: Config = toml::from_str("tab_width = 8").unwrap();
        assert_eq!(config.tab_width, 8);
        assert_eq!(config.large_file_warning_bytes, 10 * 1024 * 1024);
        assert!(toml::from_str::<Config>("unknown_key = true").is_err());
    }
}
