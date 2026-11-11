use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Editor { #[default] Auto, VsCode, Cursor, Zed, RustRover }

impl Editor {
    pub const ALL: [Self;5]=[Self::Auto,Self::VsCode,Self::Cursor,Self::Zed,Self::RustRover];
    pub fn label(self)->&'static str {
        match self {Self::Auto=>"Automatic",Self::VsCode=>"VS Code",Self::Cursor=>"Cursor",Self::Zed=>"Zed",Self::RustRover=>"RustRover"}
    }
    fn app(self)->&'static str {
        match self {Self::VsCode=>"Visual Studio Code",Self::Cursor=>"Cursor",Self::Zed=>"Zed",Self::RustRover=>"RustRover",Self::Auto=>unreachable!()}
    }
}

struct Launch { program:OsString,args:Vec<OsString>,wait:bool }

fn editor_command(editor:Editor,path:&Path,platform:&str)->Launch {
    match platform {
        "macos"=>Launch{program:"open".into(),args:vec!["-a".into(),editor.app().into(),"--".into(),path.as_os_str().into()],wait:true},
        "windows"=>Launch{program:match editor{Editor::VsCode=>"Code.exe",Editor::Cursor=>"Cursor.exe",Editor::Zed=>"zed.exe",Editor::RustRover=>"rustrover64.exe",Editor::Auto=>unreachable!()}.into(),args:vec![path.as_os_str().into()],wait:false},
        _=>Launch{program:match editor{Editor::VsCode=>"code",Editor::Cursor=>"cursor",Editor::Zed=>"zed",Editor::RustRover=>"rustrover",Editor::Auto=>unreachable!()}.into(),args:vec![path.as_os_str().into()],wait:false},
    }
}

fn run(spec:Launch)->Result<(),String> {
    let mut command=Command::new(spec.program); command.args(spec.args).stdin(Stdio::null());
    if spec.wait {
        let output=command.output().map_err(|e|e.to_string())?;
        if output.status.success(){Ok(())}else{Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())}
    } else {
        let mut child=command.stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e|e.to_string())?;
        std::thread::spawn(move ||{let _=child.wait();});
        Ok(())
    }
}

pub fn open(editor:Editor,path:&Path)->Result<String,String> {
    let path=path.canonicalize().map_err(|e|e.to_string())?;
    let choices=if editor==Editor::Auto {vec![Editor::VsCode,Editor::Cursor,Editor::Zed,Editor::RustRover]}else{vec![editor]};
    let mut last=String::new();
    for choice in choices {
        let mut spec=editor_command(choice,&path,std::env::consts::OS);
        // Installed Windows applications may not publish command-line launchers.
        #[cfg(target_os="windows")]
        {
            let suffix=match choice {Editor::VsCode=>Some("Programs/Microsoft VS Code/Code.exe"),Editor::Cursor=>Some("Programs/cursor/Cursor.exe"),_=>None};
            if let (Some(base),Some(suffix))=(std::env::var_os("LOCALAPPDATA"),suffix) {
                let executable=std::path::PathBuf::from(base).join(suffix);
                if executable.is_file(){spec.program=executable.into_os_string();}
            }
        }
        match run(spec){Ok(())=>return Ok(format!("Opened {} in {}.",path.display(),choice.label())),Err(error)=>last=error}
    }
    Err(format!("Could not open an external editor. Select an installed editor; on Linux/Windows its launcher must be available. {last}"))
}

pub fn reveal(path:&Path)->Result<String,String> {
    let path=path.canonicalize().map_err(|e|e.to_string())?;
    let folder=if path.is_dir(){path.as_path()}else{path.parent().ok_or("Missing parent folder.")?};
    let program=match std::env::consts::OS{"macos"=>"open","windows"=>"explorer.exe",_=>"xdg-open"};
    run(Launch{program:program.into(),args:vec![folder.as_os_str().into()],wait:cfg!(target_os="macos")})?;
    Ok(format!("Opened folder {}.",folder.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_path_is_one_literal_argument_on_each_platform() {
        let path=Path::new("/tmp/my Rust project; $(echo no)/Cargo.toml");
        for platform in ["macos","linux","windows"] {
            for editor in [Editor::VsCode,Editor::Cursor,Editor::Zed,Editor::RustRover] {
                let command=editor_command(editor,path,platform);
                assert_eq!(command.args.last().unwrap(),path.as_os_str());
                assert!(!["sh","bash","cmd.exe","powershell"].contains(&command.program.to_str().unwrap()));
            }
        }
    }
}
