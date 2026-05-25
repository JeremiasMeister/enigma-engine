[package]
name = "ENIGMA_PROJECT_NAME"
version = "0.1.0"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
enigma-3d = { path = "/Users/cg-jm/jm/enigma-3d" }
winit = "0.28.7"
egui = "0.23.0"
egui_glium = "0.23.0"
egui-winit = "0.23.0"
serde_json = "1.0.108"
serde = { version = "1.0.193", features = ["derive"] }