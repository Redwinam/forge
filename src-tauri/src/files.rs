use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

/// Checks that `path` lies inside `root` once both are canonicalized, so neither `..` nor a
/// symlink can reach files elsewhere, and returns the canonical path. A path that doesn't
/// exist yet is accepted when its parent directory is inside the root.
pub fn resolve_in_root(root: &Path, path: &Path) -> io::Result<PathBuf> {
    let root = fs::canonicalize(root)?;
    let resolved = match fs::canonicalize(path) {
        Ok(resolved) => resolved,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let name = match path.file_name() {
                Some(name) if path.parent().is_some() => name,
                _ => return Err(e),
            };
            fs::canonicalize(path.parent().unwrap_or(Path::new(".")))?.join(name)
        }
        Err(e) => return Err(e),
    };
    if resolved.starts_with(&root) {
        Ok(resolved)
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Path is outside the notes folder",
        ))
    }
}

/// Lists a directory with folders first, skipping hidden entries. Returned paths are built
/// from `dir` as given, so they stay comparable with the paths the UI already holds.
pub fn list_dir(dir: &Path) -> io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let path = dir.join(entry.file_name());
        entries.push(FileEntry {
            name,
            is_dir: path.is_dir(),
            path: path.to_string_lossy().to_string(),
        });
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    Ok(entries)
}

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

    fn notes_root() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("a.md"), "a").unwrap();
        fs::write(root.join("sub/b.md"), "b").unwrap();
        fs::write(dir.path().join("secret.txt"), "secret").unwrap();
        (dir, root)
    }

    #[test]
    fn accepts_paths_inside_the_root() {
        let (_dir, root) = notes_root();
        let canonical_root = fs::canonicalize(&root).unwrap();

        assert_eq!(resolve_in_root(&root, &root).unwrap(), canonical_root);
        assert_eq!(resolve_in_root(&root, &root.join("sub/b.md")).unwrap(), canonical_root.join("sub/b.md"));
        // Not created yet, but its folder is inside the root
        assert_eq!(resolve_in_root(&root, &root.join("sub/new.md")).unwrap(), canonical_root.join("sub/new.md"));
    }

    #[test]
    fn rejects_paths_outside_the_root() {
        let (dir, root) = notes_root();

        for path in [
            dir.path().join("secret.txt"),
            root.join("../secret.txt"),
            root.join("sub/../../secret.txt"),
            root.join("../notes-sibling.md"),
            dir.path().to_path_buf(),
            PathBuf::from("/"),
        ] {
            let err = resolve_in_root(&root, &path).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::PermissionDenied, "{}", path.display());
        }
    }

    #[test]
    fn rejects_a_sibling_folder_sharing_the_root_prefix() {
        let (dir, root) = notes_root();
        let sibling = dir.path().join("notes-old");
        fs::create_dir(&sibling).unwrap();
        fs::write(sibling.join("c.md"), "c").unwrap();

        assert!(resolve_in_root(&root, &sibling.join("c.md")).is_err());
    }

    #[test]
    fn rejects_missing_files_in_missing_folders() {
        let (_dir, root) = notes_root();
        assert!(resolve_in_root(&root, &root.join("nope/new.md")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_that_leave_the_root() {
        let (dir, root) = notes_root();
        std::os::unix::fs::symlink(dir.path().join("secret.txt"), root.join("link.md")).unwrap();
        std::os::unix::fs::symlink(dir.path(), root.join("escape")).unwrap();

        assert!(resolve_in_root(&root, &root.join("link.md")).is_err());
        assert!(resolve_in_root(&root, &root.join("escape/secret.txt")).is_err());
        assert!(resolve_in_root(&root, &root.join("escape/new.md")).is_err());
    }

    #[test]
    fn lists_folders_first_and_hides_dotfiles() {
        let (_dir, root) = notes_root();
        fs::write(root.join(".hidden.md"), "").unwrap();
        fs::write(root.join("0.md"), "").unwrap();

        let entries = list_dir(&root).unwrap();

        let names: Vec<_> = entries.iter().map(|e| (e.name.as_str(), e.is_dir)).collect();
        assert_eq!(names, [("sub", true), ("0.md", false), ("a.md", false)]);
        assert_eq!(entries[0].path, root.join("sub").to_string_lossy());
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
