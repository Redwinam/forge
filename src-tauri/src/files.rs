use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// Replaces the file at `path` without ever leaving it half-written: the content goes to a
/// temporary file next to it, which is flushed to disk and then renamed over the original.
/// Symlinks are written through, and the original's permissions and extended attributes are
/// carried over.
pub fn write_atomic(path: &Path, content: &[u8]) -> io::Result<()> {
    write_atomic_with(path, |file| file.write_all(content))
}

fn write_atomic_with(path: &Path, write: impl FnOnce(&mut File) -> io::Result<()>) -> io::Result<()> {
    let target = match fs::canonicalize(path) {
        Ok(target) => target,
        Err(e) if e.kind() == io::ErrorKind::NotFound => path.to_path_buf(),
        Err(e) => return Err(e),
    };
    let existing = match fs::metadata(&target) {
        Ok(meta) if meta.is_dir() => {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Is a directory"));
        }
        Ok(meta) => {
            // Renaming only needs write access to the directory, so check the file itself
            // to keep refusing read-only notes like a plain write would.
            OpenOptions::new().append(true).open(&target)?;
            Some(meta)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };

    let (tmp_path, mut file) = create_temp_beside(&target)?;
    let result = write(&mut file)
        .and_then(|()| {
            if let Some(meta) = &existing {
                fs::set_permissions(&tmp_path, meta.permissions())?;
                copy_xattrs(&target, &tmp_path);
            }
            file.sync_all()
        })
        .and_then(|()| {
            drop(file);
            fs::rename(&tmp_path, &target)
        });
    if result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    } else {
        sync_dir(&target);
    }
    result
}

fn create_temp_beside(target: &Path) -> io::Result<(PathBuf, File)> {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = match target.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    let name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Not a file path"))?
        .to_string_lossy();
    loop {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        // Dot-prefixed so the file tree never shows a temp file left behind by a crash
        let tmp_path = dir.join(format!(".{}.{}-{}.forge-tmp", name, std::process::id(), n));
        match OpenOptions::new().write(true).create_new(true).open(&tmp_path) {
            Ok(file) => return Ok((tmp_path, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
}

// Best effort: Finder tags and similar metadata live in xattrs and would vanish with the rename.
#[cfg(unix)]
fn copy_xattrs(from: &Path, to: &Path) {
    if let Ok(names) = xattr::list(from) {
        for name in names {
            if let Ok(Some(value)) = xattr::get(from, &name) {
                let _ = xattr::set(to, &name, &value);
            }
        }
    }
}

#[cfg(not(unix))]
fn copy_xattrs(_from: &Path, _to: &Path) {}

// Persist the rename itself; without this a crash can still roll the directory entry back.
#[cfg(unix)]
fn sync_dir(target: &Path) {
    if let Some(dir) = target.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        if let Ok(dir) = File::open(dir) {
            let _ = dir.sync_all();
        }
    }
}

#[cfg(not(unix))]
fn sync_dir(_target: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn replaces_existing_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "old content that is longer").unwrap();

        write_atomic(&path, "new".as_bytes()).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(entries(dir.path()), ["note.md"]);
    }

    #[test]
    fn creates_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.md");

        write_atomic(&path, "hello".as_bytes()).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn failure_mid_write_leaves_the_original_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "original").unwrap();

        let result = write_atomic_with(&path, |file| {
            file.write_all(b"half of the new")?;
            Err(io::Error::other("disk full"))
        });

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert_eq!(entries(dir.path()), ["note.md"], "temp file must be cleaned up");
    }

    #[test]
    fn refuses_to_replace_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("folder");
        fs::create_dir(&sub).unwrap();

        assert!(write_atomic(&sub, b"x").is_err());
        assert!(sub.is_dir());
        assert_eq!(entries(dir.path()), ["folder"]);
    }

    #[cfg(unix)]
    #[test]
    fn writes_through_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.md");
        let link = dir.path().join("link.md");
        fs::write(&real, "old").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();

        write_atomic(&link, b"new").unwrap();

        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read_to_string(&real).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn keeps_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();

        write_atomic(&path, b"new").unwrap();

        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o640);
    }

    #[cfg(unix)]
    #[test]
    fn refuses_read_only_files() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "original").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();

        let err = write_atomic(&path, b"new").unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert_eq!(entries(dir.path()), ["note.md"]);
    }

    #[cfg(unix)]
    #[test]
    fn keeps_extended_attributes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "old").unwrap();
        if xattr::set(&path, "user.forge.test", b"tag").is_err() {
            return; // filesystem without user xattrs (e.g. some tmpfs)
        }

        write_atomic(&path, b"new").unwrap();

        assert_eq!(xattr::get(&path, "user.forge.test").unwrap().as_deref(), Some(&b"tag"[..]));
    }
}
