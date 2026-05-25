use std::process::Command;

use crate::editor::state::ProjectState;

pub fn run_project(project: &ProjectState) {
    spawn_cargo(project, &["run"]);
}

pub fn build_project(project: &ProjectState, release: bool) {
    if release {
        spawn_cargo(project, &["build", "--release"]);
    } else {
        spawn_cargo(project, &["build"]);
    }
}

fn spawn_cargo(project: &ProjectState, args: &[&str]) {
    let output = Command::new("cargo")
        .args(args)
        .current_dir(&project.root_path)
        .output();
    match output {
        Ok(o) if o.status.success() => {
            println!("cargo {}: ok", args.join(" "));
            println!("{}", String::from_utf8_lossy(&o.stdout));
        }
        Ok(o) => {
            eprintln!("cargo {} failed", args.join(" "));
            eprintln!("{}", String::from_utf8_lossy(&o.stderr));
        }
        Err(e) => eprintln!("could not spawn cargo: {e}"),
    }
}
