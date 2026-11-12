//! Build games against the renderer used by the editor, including local engine changes.
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn renderer_source() -> Result<PathBuf, String> {
    let source = PathBuf::from(enigma_3d::postprocessing::stack::ENGINE_SOURCE_DIR);
    if !source.join("Cargo.toml").is_file() {
        return Err(format!("The editor's renderer source is unavailable at {}. Restore the source checkout before building the game.", source.display()));
    }
    Ok(source)
}

pub fn cargo_command(root: &Path, args: &[String]) -> Result<Command, String> {
    let source = renderer_source()?;
    let path = toml_edit::Value::from(source.to_string_lossy().as_ref());
    let override_source =
        format!("patch.\"https://github.com/JeremiasMeister/enigma-3d\".enigma-3d.path = {path}");
    let mut command = Command::new("cargo");
    command
        .arg("--config")
        .arg(override_source)
        .args(args)
        .current_dir(root)
        .env("CARGO_TERM_COLOR", "never");
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_and_build_use_the_editors_renderer_without_rewriting_the_project() {
        let root = tempfile::tempdir().unwrap();
        let original = "[package]\nname = \"game\"\nversion = \"0.1.0\"\n";
        std::fs::write(root.path().join("Cargo.toml"), original).unwrap();
        for action in ["run", "build"] {
            let command = cargo_command(root.path(), &[action.into()]).unwrap();
            let args: Vec<_> = command
                .get_args()
                .map(|arg| arg.to_str().unwrap())
                .collect();
            assert_eq!(args[0], "--config");
            let config: toml_edit::DocumentMut = args[1].parse().unwrap();
            let path = config["patch"]["https://github.com/JeremiasMeister/enigma-3d"]["enigma-3d"]
                ["path"]
                .as_str()
                .unwrap();
            assert_eq!(
                std::path::Path::new(path),
                std::path::Path::new(enigma_3d::postprocessing::stack::ENGINE_SOURCE_DIR)
            );
            assert_eq!(args[2], action);
            assert_eq!(command.get_current_dir(), Some(root.path()));
        }
        assert_eq!(
            std::fs::read_to_string(root.path().join("Cargo.toml")).unwrap(),
            original
        );
    }
}
