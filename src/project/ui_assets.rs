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
