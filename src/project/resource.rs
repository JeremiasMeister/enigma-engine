use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::editor::state::{ProjectState, ResourceEntry, ResourceKind};

pub fn import(src_path: &Path, kind: ResourceKind, project: &mut ProjectState) -> Result<Uuid, ImportError> {
    if !src_path.is_file() {
        return Err(ImportError::SourceNotFound);
    }
    let file_name = src_path.file_name()
        .and_then(|n| n.to_str())
        .ok_or(ImportError::BadSourcePath)?;
    let target_dir = Path::new(&project.root_path).join("src/resources").join(kind.dir_name());
    fs::create_dir_all(&target_dir).map_err(ImportError::Io)?;

    let target_path = unique_target(&target_dir, file_name);
    fs::copy(src_path, &target_path).map_err(ImportError::Io)?;

    let relative_path = format!(
        "{}/{}",
        kind.dir_name(),
        target_path.file_name().unwrap().to_str().unwrap()
    );
    let uuid = Uuid::new_v4();
    project.manifest.push(ResourceEntry {
        uuid,
        name: file_stem(&target_path),
        kind,
        relative_path,
    });
    Ok(uuid)
}

pub fn find(project: &ProjectState, uuid: Uuid) -> Option<&ResourceEntry> {
    project.manifest.iter().find(|e| e.uuid == uuid)
}

pub fn bytes(project: &ProjectState, uuid: Uuid) -> Result<Vec<u8>, BytesError> {
    let entry = find(project, uuid).ok_or(BytesError::NotFound)?;
    let path = Path::new(&project.root_path).join("src/resources").join(&entry.relative_path);
    fs::read(&path).map_err(BytesError::Io)
}

pub fn delete(project: &mut ProjectState, uuid: Uuid) -> Result<(), DeleteError> {
    let idx = project.manifest.iter().position(|e| e.uuid == uuid)
        .ok_or(DeleteError::NotFound)?;
    let entry = &project.manifest[idx];

    let src = Path::new(&project.root_path).join("src/resources").join(&entry.relative_path);
    let trash_dir = Path::new(&project.root_path).join(".trash");
    fs::create_dir_all(&trash_dir).map_err(DeleteError::Io)?;
    let trash_path = unique_target(&trash_dir, entry.relative_path.rsplit('/').next().unwrap());
    if src.exists() {
        fs::rename(&src, &trash_path).map_err(DeleteError::Io)?;
    }

    project.manifest.remove(idx);
    Ok(())
}

fn unique_target(dir: &Path, file_name: &str) -> PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() { return candidate; }
    let (stem, ext) = match file_name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (file_name.to_string(), String::new()),
    };
    for n in 2..1000 {
        let next = dir.join(format!("{stem}_{n}{ext}"));
        if !next.exists() { return next; }
    }
    dir.join(file_name)
}

fn file_stem(p: &Path) -> String {
    p.file_stem().and_then(|s| s.to_str()).unwrap_or("unnamed").to_string()
}

#[derive(Debug)]
pub enum ImportError {
    Io(std::io::Error),
    SourceNotFound,
    BadSourcePath,
}

#[derive(Debug)]
pub enum BytesError {
    NotFound,
    Io(std::io::Error),
}

#[derive(Debug)]
pub enum DeleteError {
    NotFound,
    Io(std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_project(tmp: &Path) -> ProjectState {
        ProjectState::new("test".into(), tmp.to_string_lossy().into_owned())
    }

    #[test]
    fn import_copies_file_and_adds_manifest_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());

        let src = tmp.path().join("source.png");
        fs::write(&src, b"fake-png").unwrap();

        let uuid = import(&src, ResourceKind::Texture, &mut project).unwrap();

        assert_eq!(project.manifest.len(), 1);
        let entry = &project.manifest[0];
        assert_eq!(entry.uuid, uuid);
        assert_eq!(entry.kind, ResourceKind::Texture);
        assert_eq!(entry.name, "source");
        assert_eq!(entry.relative_path, "textures/source.png");
        assert!(tmp.path().join("src/resources/textures/source.png").is_file());
    }

    #[test]
    fn import_collision_appends_suffix() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        let src = tmp.path().join("source.png");
        fs::write(&src, b"a").unwrap();

        import(&src, ResourceKind::Texture, &mut project).unwrap();
        import(&src, ResourceKind::Texture, &mut project).unwrap();

        assert_eq!(project.manifest[0].relative_path, "textures/source.png");
        assert_eq!(project.manifest[1].relative_path, "textures/source_2.png");
    }

    #[test]
    fn import_missing_source_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        let bogus = tmp.path().join("nope.png");
        match import(&bogus, ResourceKind::Texture, &mut project) {
            Err(ImportError::SourceNotFound) => (),
            other => panic!("expected SourceNotFound, got {other:?}"),
        }
    }

    #[test]
    fn bytes_reads_file_via_uuid() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        let src = tmp.path().join("source.png");
        fs::write(&src, b"hello").unwrap();
        let uuid = import(&src, ResourceKind::Texture, &mut project).unwrap();

        let data = bytes(&project, uuid).unwrap();
        assert_eq!(data, b"hello");
    }

    #[test]
    fn delete_moves_to_trash_and_removes_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        let src = tmp.path().join("source.png");
        fs::write(&src, b"x").unwrap();
        let uuid = import(&src, ResourceKind::Texture, &mut project).unwrap();

        delete(&mut project, uuid).unwrap();
        assert_eq!(project.manifest.len(), 0);
        assert!(!tmp.path().join("src/resources/textures/source.png").exists());
        assert!(tmp.path().join(".trash/source.png").exists());
    }
}
