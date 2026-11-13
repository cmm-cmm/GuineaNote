//! How file paths are shown to the user.
use std::path::{Path, PathBuf};

/// The path as the user would type it. On Windows, canonicalized document
/// paths carry the verbatim prefix (`\\?\C:\a.txt`, `\\?\UNC\server\share`);
/// it is removed for display and comparisons. Device and volume-GUID
/// verbatim paths are left alone. The document keeps its real path for all
/// filesystem operations.
pub(crate) fn display(path: &Path) -> PathBuf {
    #[cfg(windows)]
    if let Some(text) = path.to_str() {
        if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{unc}"));
        }
        if let Some(rest) = text.strip_prefix(r"\\?\")
            && rest.as_bytes().get(1) == Some(&b':')
        {
            return PathBuf::from(rest);
        }
    }
    path.to_path_buf()
}

/// `display` as text.
pub(crate) fn text(path: &Path) -> String {
    display(path).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_paths_are_shown_unchanged() {
        let path = Path::new("notes/việt.md");
        assert_eq!(display(path), path);
    }

    #[cfg(windows)]
    #[test]
    fn windows_verbatim_prefixes_are_hidden() {
        assert_eq!(text(Path::new(r"\\?\D:\demo\a.rs")), r"D:\demo\a.rs");
        assert_eq!(
            text(Path::new(r"\\?\UNC\server\share\a.rs")),
            r"\\server\share\a.rs"
        );
        let device = r"\\?\Volume{0b1c}\a.rs";
        assert_eq!(text(Path::new(device)), device);
    }
}
