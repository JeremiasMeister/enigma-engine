use crate::narrative::{CharacterLibrary, StoryNode};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone)]
pub struct Document {
    pub root: PathBuf,
    pub path: PathBuf,
    pub nodes: Vec<StoryNode>,
    pub library: CharacterLibrary,
    baseline: Option<Vec<u8>>,
    library_baseline: Option<Vec<u8>>,
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

pub fn resource_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative);
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Use a path inside project resources".into());
    }
    let resources = root.join("src/resources");
    let path = resources.join(relative);
    // Check the nearest existing ancestor to catch symlink escapes for new files too.
    let mut ancestor = path.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or("Invalid resource path")?;
    }
    let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
    if !ancestor
        .canonicalize()
        .map_err(|e| e.to_string())?
        .starts_with(&canonical_root)
    {
        return Err("Resource path escapes project through a symlink".into());
    }
    if resources.exists() {
        let base = resources.canonicalize().map_err(|e| e.to_string())?;
        if !base.starts_with(&canonical_root)
            || !ancestor
                .canonicalize()
                .map_err(|e| e.to_string())?
                .starts_with(&base)
        {
            return Err("Resource path escapes resource directory".into());
        }
    }
    Ok(path)
}

fn library_path(root: &Path) -> Result<PathBuf, String> {
    resource_path(root, "story/characters.json")
}

impl Document {
    pub fn load(root: &Path, path: &Path) -> Result<Self, String> {
        let resources = root.join("src/resources");
        let relative = path
            .strip_prefix(&resources)
            .map_err(|_| "Story must be inside project resources")?;
        let path = resource_path(root, &relative.to_string_lossy())?;
        let baseline = read_optional(&path)?;
        let bytes = baseline.as_ref().ok_or("Story file is missing")?;
        let nodes = serde_json::from_slice(bytes).map_err(|e| format!("Cannot read story: {e}"))?;
        let library_baseline = read_optional(&library_path(root)?)?;
        let library = match &library_baseline {
            Some(bytes) => serde_json::from_slice(bytes)
                .map_err(|e| format!("Cannot read character library: {e}"))?,
            None => CharacterLibrary::default(),
        };
        Ok(Self {
            root: root.into(),
            path,
            nodes,
            library,
            baseline,
            library_baseline,
        })
    }
    pub fn create(root: &Path, name: &str) -> Result<Self, String> {
        if name.is_empty()
            || name == "characters"
            || name.len() > 100
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
            || name.starts_with('.')
        {
            return Err("Use a chapter name with letters, numbers, dots, dashes or underscores (not characters)".into());
        }
        let path = resource_path(root, &format!("story/{name}.json"))?;
        if path.exists() {
            return Err("A story with this name already exists".into());
        }
        let library_baseline = read_optional(&library_path(root)?)?;
        let library = match &library_baseline {
            Some(bytes) => serde_json::from_slice(bytes)
                .map_err(|e| format!("Cannot read character library: {e}"))?,
            None => CharacterLibrary::default(),
        };
        Ok(Self {
            root: root.into(),
            path,
            nodes: vec![StoryNode {
                id: "opening".into(),
                text: "Your story begins here.".into(),
                ..StoryNode::default()
            }],
            library,
            baseline: None,
            library_baseline,
        })
    }
    pub fn dirty(&self) -> bool {
        let same_nodes = self
            .baseline
            .as_ref()
            .and_then(|b| serde_json::from_slice::<Vec<StoryNode>>(b).ok())
            .is_some_and(|n| n == self.nodes);
        let saved_library = self
            .library_baseline
            .as_ref()
            .and_then(|b| serde_json::from_slice::<CharacterLibrary>(b).ok())
            .unwrap_or_default();
        !same_nodes || saved_library != self.library
    }
    pub fn save(&mut self) -> Result<(), String> {
        let library_path = library_path(&self.root)?;
        let relative = self
            .path
            .strip_prefix(self.root.join("src/resources"))
            .map_err(|_| "Invalid story path")?;
        resource_path(&self.root, &relative.to_string_lossy())?;
        if read_optional(&self.path)? != self.baseline
            || read_optional(&library_path)? != self.library_baseline
        {
            return Err("Files changed outside the editor. Keep your draft, or reload to use the disk version.".into());
        }
        let nodes = serde_json::to_vec_pretty(&self.nodes).map_err(|e| e.to_string())?;
        let library = serde_json::to_vec_pretty(&self.library).map_err(|e| e.to_string())?;
        atomic_write(&self.path, &nodes)?;
        if let Err(error) = atomic_write(&library_path, &library) {
            let rollback = match &self.baseline {
                Some(bytes) => atomic_write(&self.path, bytes),
                None => fs::remove_file(&self.path).map_err(|e| e.to_string()),
            };
            return Err(format!(
                "Character save failed: {error}. Story rollback: {}",
                if rollback.is_ok() {
                    "succeeded"
                } else {
                    "failed; keep the draft and check disk"
                }
            ));
        }
        self.baseline = Some(nodes);
        self.library_baseline = Some(library);
        Ok(())
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing parent directory")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = parent.join(format!(".narrative-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(&temp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

pub fn discover(root: &Path) -> Result<Vec<PathBuf>, String> {
    let dir = resource_path(root, "story")?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|s| s == "json")
            && path.file_name().is_some_and(|s| s != "characters.json")
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn import_portrait(root: &Path, source: &Path) -> Result<String, String> {
    let metadata = source.metadata().map_err(|e| e.to_string())?;
    if metadata.len() > 32 * 1024 * 1024 {
        return Err("Portrait must be under 32 MB".into());
    }
    let mut reader = image::io::Reader::open(source)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::io::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|e| format!("Cannot decode portrait: {e}"))?;
    let relative = format!("portraits/{}.png", uuid::Uuid::new_v4());
    let path = resource_path(root, &relative)?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    image.save(&path).map_err(|e| e.to_string())?;
    Ok(relative)
}

#[derive(Clone, Debug)]
pub struct PresentationAsset {
    pub path: String,
    pub model: bool,
}

/// Resource files are discovered as well as manifest imports: generated projects often
/// ship artwork directly in resources without an editor manifest entry.
pub fn presentation_assets(root: &Path) -> Result<Vec<PresentationAsset>, String> {
    let base = root.join("src/resources");
    if !base.exists() {
        return Ok(Vec::new());
    }
    let canonical = base.canonicalize().map_err(|e| e.to_string())?;
    if !canonical.starts_with(root.canonicalize().map_err(|e| e.to_string())?) {
        return Err("Resource directory escapes project".into());
    }
    let mut pending = vec![base.clone()];
    let mut assets = Vec::new();
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            // Do not follow symlinks into unrelated folders or cycles.
            if kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            if kind.is_dir() {
                pending.push(path);
                continue;
            }
            let extension = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let model = matches!(extension.as_str(), "glb" | "gltf");
            if kind.is_file()
                && (model
                    || matches!(
                        extension.as_str(),
                        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tga"
                    ))
            {
                assets.push(PresentationAsset {
                    path: path
                        .strip_prefix(&base)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .replace('\\', "/"),
                    model,
                });
            }
        }
    }
    assets.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(assets)
}

/// Discover resource-relative audio paths without following directory symlinks.
pub fn audio_assets(root: &Path) -> Result<Vec<String>, String> {
    let base = root.join("src/resources");
    if !base.exists() {
        return Ok(Vec::new());
    }
    if !base
        .canonicalize()
        .map_err(|e| e.to_string())?
        .starts_with(root.canonicalize().map_err(|e| e.to_string())?)
    {
        return Err("Resource directory escapes project".into());
    }
    let mut pending = vec![base.clone()];
    let mut assets = Vec::new();
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            if kind.is_dir() {
                pending.push(path);
                continue;
            }
            let extension = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if kind.is_file() && matches!(extension.as_str(), "mp3" | "wav" | "ogg" | "flac") {
                assets.push(
                    path.strip_prefix(&base)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    assets.sort();
    Ok(assets)
}

#[cfg(test)]
mod audio_asset_tests {
    use super::*;
    #[test]
    fn audio_catalog_discovers_nested_supported_files_and_skips_other_assets() {
        let root = tempfile::tempdir().unwrap();
        assert!(audio_assets(root.path()).unwrap().is_empty());
        let base = root.path().join("src/resources/audio/dialogue");
        fs::create_dir_all(&base).unwrap();
        for name in [
            "line.MP3",
            "rain.wav",
            "theme.ogg",
            "bell.flac",
            "cover.png",
            "notes.txt",
        ] {
            fs::write(base.join(name), []).unwrap();
        }
        assert_eq!(
            audio_assets(root.path()).unwrap(),
            vec![
                "audio/dialogue/bell.flac",
                "audio/dialogue/line.MP3",
                "audio/dialogue/rain.wav",
                "audio/dialogue/theme.ogg"
            ]
        );
    }
    #[cfg(unix)]
    #[test]
    fn audio_catalog_ignores_symlinked_files_and_directories() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let base = root.path().join("src/resources");
        fs::create_dir_all(&base).unwrap();
        fs::write(outside.path().join("private.mp3"), []).unwrap();
        std::os::unix::fs::symlink(outside.path(), base.join("outside")).unwrap();
        std::os::unix::fs::symlink(outside.path().join("private.mp3"), base.join("linked.mp3"))
            .unwrap();
        assert!(audio_assets(root.path()).unwrap().is_empty());
    }
}

/// Copy a glTF/GLB and its relative buffers/images together, preserving URI paths.
pub fn import_model(root: &Path, source: &Path) -> Result<crate::narrative::ModelAsset, String> {
    if !source
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gltf") || e.eq_ignore_ascii_case("glb"))
    {
        return Err("Choose a glTF (.gltf) or binary glTF (.glb) model".into());
    }
    if source.metadata().map_err(|e| e.to_string())?.len() > 128 * 1024 * 1024 {
        return Err("Model must be under 128 MB".into());
    }
    let bytes = fs::read(source).map_err(|e| e.to_string())?;
    let gltf = gltf::Gltf::from_slice(&bytes).map_err(|e| format!("Invalid glTF: {e}"))?;
    let mut uris = std::collections::BTreeSet::new();
    for buffer in gltf.buffers() {
        if let gltf::buffer::Source::Uri(uri) = buffer.source() {
            uris.insert(uri.to_owned());
        }
    }
    for image in gltf.images() {
        if let gltf::image::Source::Uri { uri, .. } = image.source() {
            uris.insert(uri.to_owned());
        }
    }
    let source_dir = source
        .parent()
        .ok_or("Model has no parent directory")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let mut dependencies = Vec::new();
    let mut total = bytes.len() as u64;
    for uri in uris {
        if uri.starts_with("data:") {
            continue;
        }
        if uri.contains(['%', ':', '?', '#', '\\'])
            || Path::new(&uri)
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err("Model dependencies must be simple relative paths; export a self-contained GLB for this model".into());
        }
        let path = source_dir
            .join(&uri)
            .canonicalize()
            .map_err(|e| format!("Missing model dependency {uri}: {e}"))?;
        if !path.starts_with(&source_dir) {
            return Err("Model dependency escapes its source directory".into());
        }
        total += path.metadata().map_err(|e| e.to_string())?.len();
        if total > 256 * 1024 * 1024 {
            return Err("Model bundle must be under 256 MB".into());
        }
        dependencies.push((uri, path));
    }
    // Decode before creating any destination files, including referenced buffers and textures.
    gltf::import(source).map_err(|e| format!("Cannot load model: {e}"))?;
    let file_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid model filename")?;
    let relative = format!("models/{}/{file_name}", uuid::Uuid::new_v4());
    let destination = resource_path(root, &relative)?;
    let directory = destination.parent().unwrap();
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let result = (|| {
        for (uri, source) in dependencies {
            let target = directory.join(uri);
            fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::copy(source, target).map_err(|e| e.to_string())?;
        }
        fs::write(&destination, bytes).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(directory);
    }
    result?;
    Ok(crate::narrative::ModelAsset {
        path: relative,
        ..Default::default()
    })
}

/// Export a renderer-independent module for projects that do not link the editor.
pub fn export_runtime(root: &Path) -> Result<PathBuf, String> {
    let path = root.join("src/narrative_runtime.rs");
    let source = include_str!("../narrative/mod.rs")
        .split("#[cfg(test)]")
        .next()
        .unwrap();
    if path.exists() {
        let old = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        if old != source {
            return Err("narrative_runtime.rs already exists with different content; preserve it or rename it before exporting".into());
        }
    } else {
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        file.write_all(source.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn save_reopen_preserves_story_and_shared_characters() {
        let dir = tempfile::tempdir().unwrap();
        let mut doc = Document::create(dir.path(), "chapter").unwrap();
        doc.nodes[0].text = "Hello".into();
        doc.library
            .characters
            .push(crate::narrative::Character::new(
                "bera".into(),
                "Bera".into(),
            ));
        doc.save().unwrap();
        let loaded = Document::load(dir.path(), &doc.path).unwrap();
        assert_eq!(loaded.nodes[0].text, "Hello");
        assert_eq!(loaded.library.characters[0].name, "Bera");
        assert_eq!(discover(dir.path()).unwrap().len(), 1);
    }
    #[test]
    fn external_changes_block_save_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let mut doc = Document::create(dir.path(), "chapter").unwrap();
        doc.save().unwrap();
        std::fs::write(&doc.path, "external change").unwrap();
        assert!(doc.save().is_err());
        assert_eq!(
            std::fs::read_to_string(&doc.path).unwrap(),
            "external change"
        );
    }
    #[test]
    fn names_and_portrait_paths_cannot_escape_resources() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Document::create(dir.path(), "../escape").is_err());
        assert!(Document::create(dir.path(), "characters").is_err());
        assert!(resource_path(dir.path(), "../outside.png").is_err());
        assert!(resource_path(dir.path(), "/tmp/outside.png").is_err());
    }
}

#[cfg(test)]
mod model_tests {
    use super::*;
    #[test]
    fn import_gltf_copies_external_buffers_into_a_portable_bundle() {
        let root = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        fs::write(source.path().join("mesh.bin"), [0u8; 4]).unwrap();
        fs::write(
            source.path().join("hero.gltf"),
            r#"{"asset":{"version":"2.0"},"buffers":[{"uri":"mesh.bin","byteLength":4}]}"#,
        )
        .unwrap();
        let model = import_model(root.path(), &source.path().join("hero.gltf")).unwrap();
        let path = resource_path(root.path(), &model.path).unwrap();
        assert!(path.is_file());
        assert_eq!(
            fs::read(path.parent().unwrap().join("mesh.bin")).unwrap(),
            [0u8; 4]
        );
        assert!(gltf::import(&path).is_ok());
    }
    #[test]
    fn model_import_rejects_external_traversal_before_copying() {
        let root = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        fs::write(
            source.path().join("hero.gltf"),
            r#"{"asset":{"version":"2.0"},"buffers":[{"uri":"../secret.bin","byteLength":4}]}"#,
        )
        .unwrap();
        assert!(import_model(root.path(), &source.path().join("hero.gltf")).is_err());
        assert!(!root.path().join("src/resources/models").exists());
    }
}

#[cfg(test)]
mod asset_tests {
    use super::*;
    #[test]
    fn project_asset_catalog_finds_nested_images_and_models() {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().join("src/resources");
        fs::create_dir_all(base.join("textures/cast")).unwrap();
        fs::create_dir_all(base.join("models")).unwrap();
        fs::write(base.join("textures/cast/hero.PNG"), b"fixture").unwrap();
        fs::write(base.join("models/hero.glb"), b"fixture").unwrap();
        fs::write(base.join("models/notes.txt"), b"ignore").unwrap();
        let assets = presentation_assets(root.path()).unwrap();
        assert_eq!(assets.len(), 2);
        assert!(assets
            .iter()
            .any(|a| a.path == "textures/cast/hero.PNG" && !a.model));
        assert!(assets
            .iter()
            .any(|a| a.path == "models/hero.glb" && a.model));
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    #[test]
    fn another_story_saving_shared_library_blocks_stale_library_overwrite() {
        let root = tempfile::tempdir().unwrap();
        let mut first = Document::create(root.path(), "one").unwrap();
        first.save().unwrap();
        let mut second = Document::create(root.path(), "two").unwrap();
        second
            .library
            .characters
            .push(crate::narrative::Character::new(
                "hero".into(),
                "Hero".into(),
            ));
        second.save().unwrap();
        first.nodes[0].text = "Draft".into();
        assert!(first.save().is_err());
        assert!(first.dirty());
        let loaded = Document::load(root.path(), &first.path).unwrap();
        assert_eq!(loaded.library.characters[0].name, "Hero");
        assert_eq!(loaded.nodes[0].text, "Your story begins here.");
    }
    #[test]
    fn runtime_export_never_overwrites_user_code() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        let path = export_runtime(root.path()).unwrap();
        assert_eq!(export_runtime(root.path()).unwrap(), path);
        fs::write(&path, "// handwritten game code").unwrap();
        assert!(export_runtime(root.path()).is_err());
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "// handwritten game code"
        );
    }
    #[test]
    fn portraits_are_copied_as_valid_pngs_without_overwriting() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.png");
        image::RgbaImage::from_pixel(2, 3, image::Rgba([90, 100, 110, 255]))
            .save(&source)
            .unwrap();
        let first = import_portrait(root.path(), &source).unwrap();
        let second = import_portrait(root.path(), &source).unwrap();
        assert_ne!(first, second);
        let loaded = image::open(resource_path(root.path(), &first).unwrap()).unwrap();
        assert_eq!((loaded.width(), loaded.height()), (2, 3));
    }
    #[cfg(unix)]
    #[test]
    fn symlink_resource_escapes_are_rejected_and_not_catalogued() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("src/resources")).unwrap();
        fs::write(outside.path().join("secret.png"), b"outside").unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("src/resources/escape"))
            .unwrap();
        assert!(resource_path(root.path(), "escape/secret.png").is_err());
        assert!(presentation_assets(root.path()).unwrap().is_empty());
    }
}
