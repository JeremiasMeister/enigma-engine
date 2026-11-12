use crate::{editor::state::ProjectState, project::ui_assets};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

type DiscoveryResult = Result<Vec<(String, String)>, String>;

#[derive(Default)]
pub struct TextureDiscovery {
    root: Option<PathBuf>,
    next_scan: Option<Instant>,
    pending: Option<Receiver<DiscoveryResult>>,
    pub error: Option<String>,
}

impl TextureDiscovery {
    pub fn refresh(&mut self) {
        self.next_scan = None;
    }

    /// Apply results only to the owning project; never block a frame on a scan.
    pub fn poll(&mut self, ctx: &egui::Context, project: &mut ProjectState) -> usize {
        let root = PathBuf::from(&project.root_path);
        if self.root.as_ref() != Some(&root) {
            *self = Self {
                root: Some(root.clone()),
                ..Default::default()
            };
        }
        let mut added = 0;
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    match result {
                        Ok(files) => {
                            added = ui_assets::register_discovered_textures(project, files);
                            self.error = None;
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.error =
                        Some("Texture folder scan stopped. Click Reload to try again.".into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.pending.is_none() && self.next_scan.is_none_or(|when| Instant::now() >= when) {
            let (tx, rx) = mpsc::channel();
            self.pending = Some(rx);
            self.next_scan = Some(Instant::now() + Duration::from_secs(2));
            std::thread::spawn(move || {
                let _ = tx.send(ui_assets::discover_texture_files(&root));
            });
        }
        ctx.request_repaint_after(Duration::from_millis(if self.pending.is_some() {
            100
        } else {
            2000
        }));
        added
    }
}
