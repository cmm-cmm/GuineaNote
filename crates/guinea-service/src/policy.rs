use crate::{Error, Result};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Clone)]
pub struct Policy {
    pub root: PathBuf,
    pub write: bool,
    pub secrets: bool,
    denied: Vec<PathBuf>,
}
pub fn special(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}
impl Policy {
    pub fn new(root: &Path, write: bool, secrets: bool) -> Result<Self> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(Error::new(
                "INVALID_ARGUMENT",
                "Workspace must be a directory",
            ));
        }
        let denied = crate::ipc::directory()
            .ok()
            .and_then(|p| p.canonicalize().ok())
            .into_iter()
            .collect();
        Ok(Self {
            root,
            write,
            secrets,
            denied,
        })
    }
    pub fn check_write(&self) -> Result<()> {
        if self.write {
            Ok(())
        } else {
            Err(Error::new("READ_ONLY", "Write capability is disabled"))
        }
    }
    pub fn exclude(&mut self, path: &Path) -> Result<()> {
        self.denied.push(path.canonicalize()?);
        Ok(())
    }
    pub fn path(&self, relative: &str, missing: bool) -> Result<PathBuf> {
        let path = Path::new(relative);
        if path.is_absolute() || relative.contains([':', '\0']) {
            return Err(Error::new("OUT_OF_SCOPE", "Use a workspace-relative path"));
        }
        let mut current = self.root.clone();
        let components = path
            .components()
            .filter(|c| !matches!(c, Component::CurDir))
            .collect::<Vec<_>>();
        for (index, component) in components.iter().enumerate() {
            let Component::Normal(name) = component else {
                return Err(Error::new(
                    "OUT_OF_SCOPE",
                    "Parent and device paths are not allowed",
                ));
            };
            let name = name.to_string_lossy().to_lowercase();
            #[cfg(windows)]
            if name.ends_with(['.', ' '])
                || matches!(
                    name.split('.').next().unwrap_or(""),
                    "con" | "prn" | "aux" | "nul" | "conin$" | "conout$"
                )
                || ["com", "lpt"].iter().any(|prefix| {
                    name.split('.')
                        .next()
                        .unwrap_or("")
                        .strip_prefix(prefix)
                        .is_some_and(|n| {
                            matches!(
                                n,
                                "1" | "2"
                                    | "3"
                                    | "4"
                                    | "5"
                                    | "6"
                                    | "7"
                                    | "8"
                                    | "9"
                                    | "¹"
                                    | "²"
                                    | "³"
                            )
                        })
                })
            {
                return Err(Error::new(
                    "OUT_OF_SCOPE",
                    "Windows path aliases and device names are not allowed",
                ));
            }
            if !self.secrets
                && (name == ".env"
                    || name.starts_with(".env.")
                    || name == ".ssh"
                    || name == ".aws"
                    || name == ".gnupg"
                    || name.contains("credentials")
                    || name.ends_with(".pem")
                    || name.ends_with(".key")
                    || name.starts_with("id_rsa")
                    || name.starts_with("id_ed25519"))
            {
                return Err(Error::new(
                    "OUT_OF_SCOPE",
                    "Sensitive paths require an explicit secret-read grant",
                ));
            }
            current.push(component.as_os_str());
            let resolved = current.canonicalize().unwrap_or_else(|_| current.clone());
            if self.denied.iter().any(|denied| within(&resolved, denied)) {
                return Err(Error::new(
                    "OUT_OF_SCOPE",
                    "Agent session credentials cannot be read through workspace tools",
                ));
            }
            match fs::symlink_metadata(&current) {
                Ok(metadata) if special(&metadata) => {
                    return Err(Error::new(
                        "OUT_OF_SCOPE",
                        "Links and reparse points are not followed",
                    ));
                }
                Ok(_) => {}
                Err(e)
                    if missing
                        && index + 1 == components.len()
                        && e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(current)
    }
    pub fn relative(&self, path: &Path) -> Result<String> {
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| Error::new("OUT_OF_SCOPE", "Document is outside this workspace"))?;
        let relative = relative.to_string_lossy().into_owned();
        self.path(&relative, true)?;
        Ok(relative)
    }
}
fn within(path: &Path, root: &Path) -> bool {
    #[cfg(windows)]
    {
        PathBuf::from(path.to_string_lossy().to_lowercase())
            .starts_with(PathBuf::from(root.to_string_lossy().to_lowercase()))
    }
    #[cfg(not(windows))]
    {
        path.starts_with(root)
    }
}
