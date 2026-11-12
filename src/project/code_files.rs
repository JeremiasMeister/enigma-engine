use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MAX_TEXT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub relative: PathBuf,
    pub size: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TextDocument {
    pub root: PathBuf,
    pub relative: PathBuf,
    pub text: String,
    original: Vec<u8>,
    crlf: bool,
    bom: bool,
    #[serde(skip)]
    pub recovered: bool,
}

#[derive(Debug, PartialEq)]
pub enum SaveError {
    Conflict,
    Other(String),
}

const MAX_ENTRIES: usize = 10_000;
const DRAFT_DIR: &str = ".enigma-editor/drafts";

/// Resolve a relative path without traversing links, even when the link remains
/// inside the project. The returned path is anchored to the canonical root.
pub fn checked_path(root: &Path, relative: &Path, allow_missing: bool) -> Result<PathBuf, String> {
    use std::path::Component;
    let parts: Vec<_> = relative.components().collect();
    if parts.is_empty() || parts.iter().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("Choose a path inside the project without parent segments.".into());
    }
    let mut path = root.canonicalize().map_err(|e| e.to_string())?;
    if !path.is_dir() {
        return Err("Project root is not a directory.".into());
    }
    for (index, part) in parts.iter().enumerate() {
        path.push(part.as_os_str());
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(
                    "Linked files and folders are previewed in your external editor.".into(),
                )
            }
            Ok(meta) if index + 1 < parts.len() && !meta.is_dir() => {
                return Err("A parent path is not a directory.".into())
            }
            Ok(_) => {}
            Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{}: {error}", path.display())),
        }
    }
    Ok(path)
}

pub fn list(root: &Path) -> Result<Vec<FileEntry>, String> {
    fn scan(
        root: &Path,
        relative: &Path,
        depth: usize,
        budget: &mut usize,
        files: &mut Vec<FileEntry>,
    ) -> Result<(), String> {
        if depth > 16 || *budget == 0 {
            return Ok(());
        }
        let entries = std::fs::read_dir(root.join(relative)).map_err(|e| e.to_string())?;
        for entry in entries {
            if *budget == 0 {
                break;
            }
            *budget -= 1;
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            let name_text = name.to_string_lossy();
            if matches!(
                name_text.as_ref(),
                "target" | ".git" | ".trash" | "node_modules" | "enigma_logs" | ".enigma-editor"
            ) || name_text.starts_with(".enigma-save-")
            {
                continue;
            }
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            let child = relative.join(name);
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                scan(root, &child, depth + 1, budget, files)?;
            } else if kind.is_file() {
                files.push(FileEntry {
                    relative: child,
                    size: entry.metadata().map_err(|e| e.to_string())?.len(),
                });
            }
        }
        Ok(())
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    let mut budget = MAX_ENTRIES;
    scan(&root, Path::new(""), 0, &mut budget, &mut files)?;
    files.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(files)
}

pub fn read(root: &Path, relative: &Path, limit: usize) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let path = checked_path(root, relative, false)?;
    let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("This entry is not a regular file.".into());
    }
    if meta.len() > limit as u64 {
        return Err(format!(
            "File exceeds the {} KiB preview limit.",
            limit / 1024
        ));
    }
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("File grew beyond the preview limit while reading.".into());
    }
    Ok(bytes)
}

fn atomic_write(
    root: &Path,
    relative: &Path,
    bytes: &[u8],
    create_parents: bool,
    expected: Option<&[u8]>,
) -> Result<(), SaveError> {
    use std::io::Write;
    let err = SaveError::Other;
    let path = checked_path(root, relative, true).map_err(err)?;
    let parent = path
        .parent()
        .ok_or_else(|| err("Missing file parent.".into()))?;
    if create_parents {
        std::fs::create_dir_all(parent).map_err(|e| err(e.to_string()))?;
    }
    checked_path(root, relative, true).map_err(err)?;
    let permissions = match std::fs::metadata(&path) {
        Ok(meta) if !meta.is_file() => {
            return Err(err("Destination is not a regular file.".into()))
        }
        Ok(meta) if meta.permissions().readonly() => return Err(err("File is read-only.".into())),
        Ok(meta) => Some(meta.permissions()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(err(e.to_string())),
    };
    let temp = parent.join(format!(".enigma-save-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| err(e.to_string()))?;
        file.write_all(bytes).map_err(|e| err(e.to_string()))?;
        if let Some(permissions) = permissions {
            file.set_permissions(permissions)
                .map_err(|e| err(e.to_string()))?;
        }
        file.sync_all().map_err(|e| err(e.to_string()))?;
        drop(file);
        checked_path(root, relative, true).map_err(err)?;
        if let Some(expected) = expected {
            let current = read(root, relative, MAX_TEXT_BYTES).map_err(err)?;
            if current != expected {
                return Err(SaveError::Conflict);
            }
        }
        std::fs::rename(&temp, &path).map_err(|e| err(e.to_string()))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict => write!(
                f,
                "The file changed on disk. Reload it or explicitly overwrite the disk version."
            ),
            Self::Other(message) => f.write_str(message),
        }
    }
}

impl TextDocument {
    pub fn open(root: &Path, relative: &Path) -> Result<Self, String> {
        let document = Self {
            root: root.canonicalize().map_err(|e| e.to_string())?,
            relative: relative.into(),
            text: String::new(),
            original: Vec::new(),
            crlf: false,
            bom: false,
            recovered: false,
        };
        let source_path = checked_path(&document.root, relative, false)?;
        if !std::fs::metadata(source_path)
            .map_err(|e| e.to_string())?
            .is_file()
        {
            return Err("This entry is not a regular file.".into());
        }
        let draft = document.draft_path();
        let path = checked_path(&document.root, &draft, true)?;
        if path.exists() {
            let bytes = read(&document.root, &draft, 12 * MAX_TEXT_BYTES)?;
            let mut recovered: Self =
                serde_json::from_slice(&bytes).map_err(|e| format!("Cannot recover draft: {e}"))?;
            if recovered.relative != relative
                || recovered.root != document.root
                || recovered.text.len() > MAX_TEXT_BYTES
                || recovered.original.len() > MAX_TEXT_BYTES
                || recovered.text.contains('\0')
                || recovered.bom != recovered.original.starts_with(&[0xef, 0xbb, 0xbf])
                || std::str::from_utf8(&recovered.original).map_or(true, |text| text.contains('\0'))
            {
                return Err("Recovery draft does not match this document or is invalid.".into());
            }
            recovered.recovered = true;
            return Ok(recovered);
        }
        Self::open_disk(root, relative)
    }

    /// Read only the source version for an explicitly requested reload.
    pub fn open_disk(root: &Path, relative: &Path) -> Result<Self, String> {
        let original = read(root, relative, MAX_TEXT_BYTES)?;
        let bom = original.starts_with(&[0xef, 0xbb, 0xbf]);
        let source = std::str::from_utf8(&original[if bom { 3 } else { 0 }..])
            .map_err(|_| "File is not UTF-8 text.".to_string())?;
        if source.contains('\0') {
            return Err("Binary files cannot be edited as text.".into());
        }
        let crlf = source.contains("\r\n");
        let document = Self {
            root: root.canonicalize().map_err(|e| e.to_string())?,
            relative: relative.into(),
            text: source.replace("\r\n", "\n"),
            original,
            crlf,
            bom,
            recovered: false,
        };
        Ok(document)
    }
    fn encoded(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        if self.bom {
            bytes.extend_from_slice(&[0xef, 0xbb, 0xbf]);
        }
        if self.crlf {
            bytes.extend_from_slice(
                self.text
                    .replace("\r\n", "\n")
                    .replace('\n', "\r\n")
                    .as_bytes(),
            );
        } else {
            bytes.extend_from_slice(self.text.as_bytes());
        }
        bytes
    }
    pub fn editing_limits(&self) -> (usize, bool) {
        (MAX_TEXT_BYTES - if self.bom { 3 } else { 0 }, self.crlf)
    }
    pub fn dirty(&self) -> bool {
        let source = &self.original[if self.bom { 3 } else { 0 }..];
        std::str::from_utf8(source)
            .map(|text| self.text != text.replace("\r\n", "\n"))
            .unwrap_or(true)
    }
    pub fn save(&mut self, overwrite: bool) -> Result<(), SaveError> {
        let bytes = self.encoded();
        if bytes.len() > MAX_TEXT_BYTES || self.text.contains('\0') {
            return Err(SaveError::Other(
                "Text exceeds 1 MiB or contains binary characters.".into(),
            ));
        }
        // Even explicit overwrite requires the original regular file to exist.
        read(&self.root, &self.relative, MAX_TEXT_BYTES).map_err(SaveError::Other)?;
        atomic_write(
            &self.root,
            &self.relative,
            &bytes,
            false,
            if overwrite {
                None
            } else {
                Some(&self.original)
            },
        )?;
        self.original = bytes;
        self.recovered = false;
        self.remove_draft()
            .map_err(|e| SaveError::Other(format!("File saved, but recovery cleanup failed: {e}")))
    }
    fn draft_path(&self) -> PathBuf {
        // Stable FNV-1a key; the payload also verifies the full path to detect a collision.
        let mut hash = 0xcbf29ce484222325u64;
        for byte in self.relative.as_os_str().to_string_lossy().as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        Path::new(DRAFT_DIR).join(format!("{hash:016x}.json"))
    }
    pub fn store_draft(&self) -> Result<(), String> {
        if !self.dirty() {
            return self.remove_draft();
        }
        if self.text.len() > MAX_TEXT_BYTES {
            return Err(
                "Draft exceeds the 1 MiB editing limit. Shorten the text before closing.".into(),
            );
        }
        let path = self.draft_path();
        let resolved = checked_path(&self.root, &path, true)?;
        if resolved.exists() {
            let previous: Self =
                serde_json::from_slice(&read(&self.root, &path, 12 * MAX_TEXT_BYTES)?)
                    .map_err(|e| format!("Invalid recovery file: {e}"))?;
            if previous.root != self.root || previous.relative != self.relative {
                return Err("Recovery filename conflicts with another document.".into());
            }
        }
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        atomic_write(&self.root, &path, &bytes, true, None).map_err(|e| e.to_string())?;
        // Recovery files are local editor state, not project source. Respect an
        // existing ignore file and never replace it or follow a link.
        use std::io::Write;
        let ignore = checked_path(&self.root, Path::new(".enigma-editor/.gitignore"), true)?;
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(ignore)
        {
            Ok(mut file) => file.write_all(b"*\n").map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn remove_draft(&self) -> Result<(), String> {
        let path = checked_path(&self.root, &self.draft_path(), true)?;
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        fs::write(root.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        root
    }
    #[test]
    fn edit_save_preserves_crlf_bom_and_reopens() {
        let root = fixture();
        fs::write(
            root.path().join("src/main.rs"),
            b"\xef\xbb\xbffn main() {}\r\n",
        )
        .unwrap();
        let mut document = TextDocument::open(root.path(), Path::new("src/main.rs")).unwrap();
        assert_eq!(document.text, "fn main() {}\n");
        assert!(!document.dirty());
        document.text.push_str("// changed\n");
        document.save(false).unwrap();
        assert!(!document.dirty());
        assert_eq!(
            fs::read(root.path().join("src/main.rs")).unwrap(),
            b"\xef\xbb\xbffn main() {}\r\n// changed\r\n"
        );
        assert_eq!(
            TextDocument::open(root.path(), Path::new("src/main.rs"))
                .unwrap()
                .text,
            document.text
        );
    }
    #[test]
    fn external_changes_require_explicit_overwrite() {
        let root = fixture();
        let mut document = TextDocument::open(root.path(), Path::new("src/main.rs")).unwrap();
        document.text = "// mine".into();
        fs::write(root.path().join("src/main.rs"), "// external").unwrap();
        assert_eq!(document.save(false), Err(SaveError::Conflict));
        assert_eq!(
            fs::read_to_string(root.path().join("src/main.rs")).unwrap(),
            "// external"
        );
        assert!(document.dirty());
        document.save(true).unwrap();
        assert_eq!(
            fs::read_to_string(root.path().join("src/main.rs")).unwrap(),
            "// mine"
        );
    }
    #[test]
    fn drafts_recover_without_changing_source_or_hiding_external_conflicts() {
        let root = fixture();
        let mut document = TextDocument::open(root.path(), Path::new("src/main.rs")).unwrap();
        document.text = "// unsaved draft\n".into();
        document.store_draft().unwrap();
        assert_eq!(
            fs::read_to_string(root.path().join("src/main.rs")).unwrap(),
            "fn main() {}\n"
        );
        fs::write(root.path().join("src/main.rs"), "// other editor").unwrap();
        let mut recovered = TextDocument::open(root.path(), Path::new("src/main.rs")).unwrap();
        assert!(recovered.recovered);
        assert_eq!(recovered.text, document.text);
        assert_eq!(recovered.save(false), Err(SaveError::Conflict));
        recovered.save(true).unwrap();
        assert!(
            !TextDocument::open(root.path(), Path::new("src/main.rs"))
                .unwrap()
                .recovered
        );
    }
    #[test]
    fn browsing_omits_build_and_recovery_directories_and_rejects_escapes() {
        let root = fixture();
        for directory in ["target", ".git", ".enigma-editor", "enigma_logs"] {
            fs::create_dir(root.path().join(directory)).unwrap();
            fs::write(root.path().join(directory).join("hidden"), "data").unwrap();
        }
        let files = list(root.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative, Path::new("src/main.rs"));
        assert!(read(root.path(), Path::new("../outside"), 100).is_err());
        assert!(read(root.path(), &root.path().join("src/main.rs"), 100).is_err());
    }
    #[test]
    fn binary_and_oversized_text_are_never_opened_as_editable_text() {
        let root = fixture();
        let path = root.path().join("src/main.rs");
        fs::write(&path, b"a\0b").unwrap();
        assert!(TextDocument::open(root.path(), Path::new("src/main.rs")).is_err());
        fs::write(&path, vec![b'x'; MAX_TEXT_BYTES + 1]).unwrap();
        assert!(TextDocument::open(root.path(), Path::new("src/main.rs")).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn linked_files_and_draft_destinations_are_rejected() {
        let root = fixture();
        let outside = fixture();
        std::os::unix::fs::symlink(
            outside.path().join("src/main.rs"),
            root.path().join("linked.rs"),
        )
        .unwrap();
        assert!(read(root.path(), Path::new("linked.rs"), 100).is_err());
        let mut document = TextDocument::open(root.path(), Path::new("src/main.rs")).unwrap();
        document.text.push('x');
        std::os::unix::fs::symlink(outside.path(), root.path().join(".enigma-editor")).unwrap();
        assert!(document.store_draft().is_err());
        assert!(!outside.path().join("drafts").exists());
    }
    #[cfg(unix)]
    #[test]
    fn saving_preserves_existing_file_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let root = fixture();
        let path = root.path().join("src/main.rs");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        let mut document = TextDocument::open(root.path(), Path::new("src/main.rs")).unwrap();
        document.text.push(' ');
        document.save(false).unwrap();
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }
    #[test]
    fn invalid_recovery_encoding_is_rejected_without_panicking() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("main.rs"), "original").unwrap();
        let mut document = TextDocument::open(root.path(), Path::new("main.rs")).unwrap();
        document.text = "draft".into();
        document.store_draft().unwrap();
        let path = root.path().join(document.draft_path());
        let mut payload = serde_json::to_value(&document).unwrap();
        payload["bom"] = true.into();
        payload["original"] = serde_json::json!([]);
        std::fs::write(path, serde_json::to_vec(&payload).unwrap()).unwrap();
        assert!(TextDocument::open(root.path(), Path::new("main.rs")).is_err());
    }

    #[test]
    fn valid_draft_recovers_when_external_source_becomes_binary_or_oversized() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("main.rs");
        std::fs::write(&path, "original").unwrap();
        let mut document = TextDocument::open(root.path(), Path::new("main.rs")).unwrap();
        document.text = "my draft".into();
        document.store_draft().unwrap();
        for external in [vec![0xff, 0], vec![b'x'; MAX_TEXT_BYTES + 1]] {
            std::fs::write(&path, &external).unwrap();
            let mut recovered = TextDocument::open(root.path(), Path::new("main.rs")).unwrap();
            assert_eq!(recovered.text, "my draft");
            assert!(recovered.save(false).is_err());
            assert_eq!(std::fs::read(&path).unwrap(), external);
        }
    }

    #[test]
    fn mixed_newlines_start_clean_and_drafts_are_git_ignored() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("main.rs"), "one\r\ntwo\n").unwrap();
        let mut document = TextDocument::open(root.path(), Path::new("main.rs")).unwrap();
        assert!(!document.dirty());
        document.text.push_str("three\n");
        document.store_draft().unwrap();
        assert_eq!(
            std::fs::read_to_string(root.path().join(".enigma-editor/.gitignore")).unwrap(),
            "*\n"
        );
    }
}
