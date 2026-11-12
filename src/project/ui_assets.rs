use crate::ui_runtime::is_resource_path;
use image::{io::Reader, ImageFormat};
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;

const MAX_SIDE: u32 = 4096;
const MAX_RGBA_BYTES: usize = 64 * 1024 * 1024;

pub struct DecodedTexture {
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
}

/// Discover artwork copied into the managed texture folder without decoding it.
/// Reuse the bounded file walk, which skips links and non-regular files.
pub fn discover_texture_files(root: &Path) -> Result<Vec<(String, String)>, String> {
    let directory =
        super::code_files::checked_path(root, Path::new("src/resources/textures"), true)?;
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for file in super::code_files::list(&directory)? {
        let extension = file
            .relative
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(
            extension.as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tga"
        ) {
            continue;
        }
        let relative = Path::new("textures")
            .join(&file.relative)
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        if !is_resource_path(&relative) {
            continue;
        }
        let name = file
            .relative
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        found.push((name, relative));
    }
    Ok(found)
}

pub fn register_discovered_textures(
    project: &mut crate::editor::state::ProjectState,
    files: Vec<(String, String)>,
) -> usize {
    use crate::editor::state::{ResourceEntry, ResourceKind};
    let mut paths: std::collections::HashSet<_> = project
        .manifest
        .iter()
        .map(|entry| entry.relative_path.clone())
        .collect();
    let mut added = 0;
    for (name, relative_path) in files {
        if paths.contains(&relative_path) {
            continue;
        }
        // The file may have been removed while the background scan was running.
        let source = Path::new("src/resources").join(&relative_path);
        if !is_resource_path(&relative_path)
            || !super::code_files::checked_path(Path::new(&project.root_path), &source, false)
                .is_ok_and(|path| path.is_file())
        {
            continue;
        }
        if paths.insert(relative_path.clone()) {
            project.manifest.push(ResourceEntry {
                uuid: uuid::Uuid::new_v4(),
                name,
                kind: ResourceKind::Texture,
                relative_path,
            });
            added += 1;
        }
    }
    added
}

pub fn load(root: &Path, relative_path: &str) -> Result<DecodedTexture, String> {
    if !is_resource_path(relative_path) {
        return Err(
            "Texture path must be relative to src/resources without parent segments.".into(),
        );
    }
    let result = (|| {
        let resources = root.canonicalize()?.join("src/resources");
        if resources.canonicalize()? != resources {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "src/resources must be inside the project",
            ));
        }
        let path = resources.join(relative_path).canonicalize()?;
        if !path.starts_with(&resources) || !path.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "texture is outside src/resources or is not a file",
            ));
        }
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_RGBA_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    })();
    let bytes = result.map_err(|err| format!("Cannot read texture {relative_path:?}: {err}"))?;
    decode(&bytes).map_err(|err| format!("Cannot load texture {relative_path:?}: {err}"))
}

pub fn decode(bytes: &[u8]) -> Result<DecodedTexture, String> {
    if bytes.len() > MAX_RGBA_BYTES {
        return Err("Image file exceeds the 64 MiB limit.".into());
    }
    let format = match image::guess_format(bytes) {
        Ok(
            format @ (ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::Bmp),
        ) => format,
        // TGA v1 has no signature. Check its header before using the TGA decoder.
        Err(_) if plausible_tga(bytes) => ImageFormat::Tga,
        _ => return Err("Expected a PNG, JPEG, GIF, BMP, or TGA image.".into()),
    };
    let reader = || Reader::with_format(Cursor::new(bytes), format);
    let (width, height) = reader()
        .into_dimensions()
        .map_err(|err| format!("Cannot read image dimensions: {err}"))?;
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(format!(
            "Image is {width} × {height}. Use dimensions up to {MAX_SIDE} × {MAX_SIDE}."
        ));
    }

    let mut image_reader = reader();
    let mut limits = image::io::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    // Some formats can decode to 16-bit RGBA before conversion to 8-bit RGBA.
    limits.max_alloc = Some(2 * MAX_RGBA_BYTES as u64);
    image_reader.limits(limits);
    let rgba = image_reader
        .decode()
        .map_err(|err| format!("Cannot decode image: {err}"))?
        .into_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    if size != [width as usize, height as usize] || rgba.len() > MAX_RGBA_BYTES {
        return Err("Decoded image exceeds its stated dimensions or the RGBA byte limit.".into());
    }
    Ok(DecodedTexture {
        size,
        rgba: rgba.into_raw(),
    })
}

fn plausible_tga(bytes: &[u8]) -> bool {
    bytes.len() >= 18
        && bytes[1] <= 1
        && matches!(bytes[2], 1..=3 | 9..=11)
        && matches!(bytes[16], 8 | 15 | 16 | 24 | 32)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_existing_texture_files_including_nested_folders() {
        let root = tempfile::tempdir().unwrap();
        let textures = root.path().join("src/resources/textures");
        std::fs::create_dir_all(textures.join("icons")).unwrap();
        for name in ["worldmap.png", "icons/portrait.PNG", "notes.txt"] {
            std::fs::write(textures.join(name), b"fixture").unwrap();
        }
        let files = discover_texture_files(root.path()).unwrap();
        assert_eq!(
            files,
            vec![
                ("portrait".into(), "textures/icons/portrait.PNG".into()),
                ("worldmap".into(), "textures/worldmap.png".into())
            ]
        );
    }

    #[test]
    fn discovered_textures_join_the_manifest_once_and_persist_on_save() {
        use crate::editor::state::{ProjectState, ResourceKind};
        let root = tempfile::tempdir().unwrap();
        let mut project = ProjectState::new("Art".into(), root.path().to_string_lossy().into());
        let source = root.path().join("imported.png");
        std::fs::write(&source, b"fixture").unwrap();
        let imported =
            crate::project::resource::import(&source, ResourceKind::Texture, &mut project).unwrap();
        std::fs::write(
            root.path().join("src/resources/textures/worldmap.png"),
            b"fixture",
        )
        .unwrap();
        project.manifest[0].name = "Custom alias".into();
        let files = vec![
            ("imported".into(), "textures/imported.png".into()),
            ("worldmap".into(), "textures/worldmap.png".into()),
        ];
        assert_eq!(register_discovered_textures(&mut project, files.clone()), 1);
        assert_eq!(register_discovered_textures(&mut project, files), 0);
        assert_eq!(project.manifest.len(), 2);
        assert_eq!(project.manifest[0].uuid, imported);
        assert_eq!(project.manifest[0].name, "Custom alias");
        crate::project::write_project_file(root.path().to_str().unwrap(), &project).unwrap();
        let reopened: ProjectState = serde_json::from_str(
            &std::fs::read_to_string(root.path().join("enigma_project.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(reopened.manifest, project.manifest);
        assert_eq!(reopened.manifest[1].relative_path, "textures/worldmap.png");
    }

    #[test]
    fn discovery_ignores_links_and_does_not_restore_a_deleted_asset() {
        let root = tempfile::tempdir().unwrap();
        let textures = root.path().join("src/resources/textures");
        std::fs::create_dir_all(&textures).unwrap();
        let path = textures.join("removed.png");
        std::fs::write(&path, b"fixture").unwrap();
        let files = discover_texture_files(root.path()).unwrap();
        std::fs::remove_file(path).unwrap();
        let mut project = crate::editor::state::ProjectState::new(
            "Art".into(),
            root.path().to_string_lossy().into(),
        );
        assert_eq!(register_discovered_textures(&mut project, files), 0);
        #[cfg(unix)]
        {
            let external = root.path().join("outside.png");
            std::fs::write(&external, b"fixture").unwrap();
            std::os::unix::fs::symlink(external, textures.join("linked.png")).unwrap();
            assert!(discover_texture_files(root.path()).unwrap().is_empty());
        }
    }

    #[test]
    fn decode_rejects_images_exceeding_dimensions_and_invalid_bytes() {
        let mut encoded = Vec::new();
        image::DynamicImage::new_rgba8(4097, 1)
            .write_to(
                &mut std::io::Cursor::new(&mut encoded),
                image::ImageOutputFormat::Png,
            )
            .unwrap();
        assert!(decode(&encoded).is_err());
        assert!(decode(b"not an image").is_err());
    }

    #[test]
    fn load_is_confined_to_existing_resources() {
        let root = tempfile::tempdir().unwrap();
        let resources = root.path().join("src/resources");
        std::fs::create_dir_all(&resources).unwrap();
        let path = resources.join("pixel.png");
        image::RgbaImage::from_pixel(2, 1, image::Rgba([23, 34, 45, 255]))
            .save(&path)
            .unwrap();
        let decoded = load(root.path(), "pixel.png").unwrap();
        assert_eq!(decoded.size, [2, 1]);
        assert_eq!(decoded.rgba, [23, 34, 45, 255, 23, 34, 45, 255]);
        for forbidden in [
            "missing.png",
            "../resources/pixel.png",
            path.to_str().unwrap(),
        ] {
            assert!(load(root.path(), forbidden).is_err(), "{forbidden}");
        }
        #[cfg(unix)]
        {
            let outside = root.path().join("outside.png");
            std::fs::copy(&path, &outside).unwrap();
            std::os::unix::fs::symlink(outside, resources.join("external.png")).unwrap();
            assert!(load(root.path(), "external.png").is_err());
        }
    }
}
