
pub const CARGO_TOML: &str = include_str!("Cargo.toml.resource");
pub const MAIN_RS: &str = include_str!("main.rs.resource");
pub const ICON: &'static [u8] = include_bytes!("icon.png.resource");

#[derive(Clone)]
pub struct BinaryResource {
    pub name: String,
    pub data: Vec<u8>,
}

impl BinaryResource {
    pub fn new(name: &str, data: Vec<u8>) -> Self {
        BinaryResource {
            name: name.to_string(),
            data,
        }
    }
}

#[derive(Clone)]
pub struct TextResource {
    pub name: String,
    pub data: String,
}

impl TextResource {
    pub fn new(name: &str, data: &str) -> Self {
        TextResource {
            name: name.to_string(),
            data: data.to_string(),
        }
    }
}

pub fn import_resource_binary(name: &str, path: &str) -> BinaryResource {
    let data = std::fs::read(path);
    match data {
        Ok(data) => {
            return BinaryResource::new(name, data);
        },
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
        }
    }
    BinaryResource::new(name, Vec::new())
}

pub fn import_resource_text(name: &str, path: &str) -> TextResource {
    let data = std::fs::read_to_string(path);
    match data {
        Ok(data) => {
            return TextResource::new(name, &data);
        },
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
        }
    }
    TextResource::new(name, "")
}