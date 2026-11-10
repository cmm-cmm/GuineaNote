use std::{
    fs::{self, File, Metadata, OpenOptions},
    io,
    path::Path,
};

pub(crate) fn special(meta: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}
pub(crate) fn open_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || special(&meta) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Links and special files are refused",
        ));
    }
    Ok(file)
}
pub(crate) fn identity(file: &File) -> io::Result<[u64; 2]> {
    #[cfg(windows)]
    {
        let info = winapi_util::file::information(file)?;
        Ok([info.volume_serial_number(), info.file_index()])
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let m = file.metadata()?;
        Ok([m.dev(), m.ino()])
    }
}
pub(crate) fn links(file: &File) -> io::Result<u64> {
    #[cfg(windows)]
    {
        Ok(winapi_util::file::information(file)?.number_of_links())
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(file.metadata()?.nlink())
    }
}
pub(crate) fn same_object(path: &Path, expected: [u64; 2]) -> io::Result<bool> {
    Ok(identity(&open_file(path)?)? == expected)
}

/// Preserve metadata before commit. Any failure aborts the save.
pub(crate) fn preserve(
    source: &File,
    source_path: &Path,
    target: &File,
    target_path: &Path,
) -> io::Result<()> {
    #[cfg(windows)]
    {
        use windows_permissions::{
            constants::{SeObjectType, SecurityInformation},
            wrappers,
        };
        let info =
            SecurityInformation::Owner | SecurityInformation::Group | SecurityInformation::Dacl;
        let sd = wrappers::GetNamedSecurityInfo(
            source_path.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            info,
        )?;
        // A protected copy retains the effective ACL instead of allowing an
        // unrelated inherited ACL on the temporary file to broaden access.
        wrappers::SetNamedSecurityInfo(
            target_path.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            info | SecurityInformation::ProtectedDacl,
            sd.owner(),
            sd.group(),
            sd.dacl(),
            None,
        )?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        use xattr::FileExt;
        let original = source.metadata()?;
        let replacement = target.metadata()?;
        if original.uid() != replacement.uid() || original.gid() != replacement.gid() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Replacement cannot preserve ownership; use Save As",
            ));
        }
        for name in source.list_xattr()? {
            if let Some(value) = source.get_xattr(&name)? {
                target.set_xattr(&name, &value)?;
            }
        }
        let _ = (source_path, target_path);
    }
    target.set_permissions(source.metadata()?.permissions())
}

/// Owner-only recovery storage; do not silently accept links or foreign owners.
pub(crate) fn private(path: &Path, directory: bool) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if special(&meta) || (directory != meta.is_dir()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Recovery paths must not be links",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if meta.uid() != tempfile::tempfile()?.metadata()?.uid() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Recovery path belongs to another user",
            ));
        }
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if directory { 0o700 } else { 0o600 }),
        )?;
    }
    #[cfg(windows)]
    {
        use windows_permissions::{
            LocalBox, SecurityDescriptor,
            constants::{SeObjectType, SecurityInformation},
            utilities, wrappers,
        };
        let sid = utilities::current_process_sid()?;
        let old = wrappers::GetNamedSecurityInfo(
            path.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            SecurityInformation::Owner,
        )?;
        if old.owner() != Some(&*sid) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Recovery path belongs to another user",
            ));
        }
        let flags = if directory { "OICI" } else { "" };
        let sd: LocalBox<SecurityDescriptor> = format!("D:P(A;{flags};FA;;;{sid})").parse()?;
        wrappers::SetNamedSecurityInfo(
            path.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
            None,
            None,
            sd.dacl(),
            None,
        )?;
    }
    Ok(())
}
