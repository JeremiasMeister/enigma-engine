use serde::{Deserialize, Serialize};

pub const CARGO_TOML: &str = include_str!("Cargo.toml.resource");
pub const MAIN_RS: &str = include_str!("main.rs.resource");
pub const ICON: &'static [u8] = include_bytes!("icon.png.resource");

#[derive(Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Texture,
    Model,
    Audio,
    Shader,
    Other,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BinaryResource {
    pub name: String,
    pub data: Vec<u8>,
    pub resource_type: ResourceType,
}

impl BinaryResource {
    pub fn new(name: &str, data: Vec<u8>, resource_type: ResourceType) -> Self {
        BinaryResource {
            name: name.to_string(),
            data,
            resource_type
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TextResource {
    pub name: String,
    pub data: String,
    pub resource_type: ResourceType
}

impl TextResource {
    pub fn new(name: &str, data: &str, resource_type: ResourceType) -> Self {
        TextResource {
            name: name.to_string(),
            data: data.to_string(),
            resource_type,
        }
    }
}

pub fn import_resource_binary(name: &str, path: &str, resource_type: ResourceType) -> BinaryResource {
    let data = std::fs::read(path);
    match data {
        Ok(data) => {
            return BinaryResource::new(name, data, resource_type);
        },
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
        }
    }
    BinaryResource::new(name, Vec::new(), resource_type)
}

pub fn import_resource_text(name: &str, path: &str, resource_type: ResourceType) -> TextResource {
    let data = std::fs::read_to_string(path);
    match data {
        Ok(data) => {
            return TextResource::new(name, &data, resource_type);
        },
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
        }
    }
    TextResource::new(name, "", resource_type)
}