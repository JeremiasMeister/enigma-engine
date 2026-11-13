mod characters;
mod graph;
mod inspector;
mod preview;

use crate::editor::theme;
use crate::narrative::{Character, CharacterLibrary, PlayState, StoryNode};
use crate::project::narrative::{self as storage, Document};
use egui::{Context, RichText, TextureHandle};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Default, PartialEq)]
enum Mode {
    #[default]
    Story,
    Graph,
    Characters,
    Dependencies,
    Preview,
}
#[derive(Clone, PartialEq)]
struct Snapshot {
    nodes: Vec<StoryNode>,
    library: CharacterLibrary,
}
enum Pending {
    Load(PathBuf, PathBuf),
    New(PathBuf, String),
}

pub struct NarrativeEditor {
    document: Option<Document>,
    project_root: Option<PathBuf>,
    files: Vec<PathBuf>,
    assets: Vec<storage::PresentationAsset>,
    audio_files: Vec<String>,
    imports: Vec<(PathBuf, String, bool)>,
    mode: Mode,
    selected: usize,
    character: usize,
    search: String,
    new_name: String,
    creating: bool,
    pending: Option<Pending>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last_edit: f64,
    pub status: String,
    error: Option<String>,
    pan: egui::Vec2,
    zoom: f32,
    play: Option<PlayState>,
    events: Vec<String>,
    play_history: Vec<(PlayState, Vec<String>)>,
    flag_input: String,
    attribute_input: String,
    model_input: String,
    rename: String,
    rename_selection: String,
    textures: BTreeMap<String, Result<TextureHandle, String>>,
}
impl Default for NarrativeEditor {
    fn default() -> Self {
        Self {
            document: None,
            project_root: None,
            files: Vec::new(),
            assets: Vec::new(),
            audio_files: Vec::new(),
            imports: Vec::new(),
            mode: Mode::Story,
            selected: 0,
            character: 0,
            search: String::new(),
            new_name: "chapter_01".into(),
            creating: false,
            pending: None,
            undo: Vec::new(),
            redo: Vec::new(),
            last_edit: -10.0,
            status: String::new(),
            error: None,
            pan: egui::vec2(30.0, 30.0),
            zoom: 1.0,
            play: None,
            events: Vec::new(),
            play_history: Vec::new(),
            flag_input: String::new(),
            attribute_input: String::new(),
            model_input: "default".into(),
            rename: String::new(),
            rename_selection: String::new(),
            textures: BTreeMap::new(),
        }
    }
}
impl NarrativeEditor {
    pub fn dirty(&self) -> bool {
        self.document.as_ref().is_some_and(Document::dirty)
    }
    /// Builds must not silently use an older story or an invalid draft.
    pub fn save_project(&mut self, root: &Path) -> Result<(), String> {
        let Some(doc) = &mut self.document else {
            return Ok(());
        };
        if doc.root != root {
            return Ok(());
        }
        let errors: Vec<_> = crate::narrative::validate(&doc.nodes, &doc.library)
            .into_iter()
            .filter(|issue| issue.error)
            .map(|issue| format!("{}: {}", issue.node, issue.message))
            .collect();
        if !errors.is_empty() {
            return Err(format!("Narrative validation: {}", errors.join("; ")));
        }
        if doc.dirty() {
            doc.save()?;
        }
        Ok(())
    }
    pub fn save(&mut self) {
        if let Some(doc) = &mut self.document {
            match doc.save() {
                Ok(()) => {
                    let errors = crate::narrative::validate(&doc.nodes, &doc.library)
                        .iter()
                        .filter(|issue| issue.error)
                        .count();
                    self.status = if errors > 0 {
                        format!("Draft saved with {errors} validation errors. Open Dependencies to resolve them before playing.")
                    } else {
                        format!(
                            "Saved {} and character library",
                            doc.path.file_name().unwrap_or_default().to_string_lossy()
                        )
                    };
                    self.error = None;
                }
                Err(e) => self.error = Some(e),
            }
        }
        self.refresh();
    }
    pub fn register_imports(&mut self, project: &mut crate::editor::state::ProjectState) -> bool {
        let mut changed = false;
        self.imports.retain(|(root, path, model)| {
            if root != Path::new(&project.root_path) {
                return true;
            }
            if !project
                .manifest
                .iter()
                .any(|entry| entry.relative_path == *path)
            {
                project.manifest.push(crate::editor::state::ResourceEntry {
                    uuid: uuid::Uuid::new_v4(),
                    name: Path::new(path)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    kind: if *model {
                        crate::editor::state::ResourceKind::Model
                    } else {
                        crate::editor::state::ResourceKind::Texture
                    },
                    relative_path: path.clone(),
                });
                changed = true;
            }
            false
        });
        changed
    }
    fn reload_assets(&mut self, root: &Path) {
        match storage::audio_assets(root) {
            Ok(files) => self.audio_files = files,
            Err(error) => {
                self.audio_files.clear();
                self.error = Some(error);
            }
        }
        match storage::presentation_assets(root) {
            Ok(assets) => self.assets = assets,
            Err(e) => self.error = Some(e),
        }
    }
    fn refresh(&mut self) {
        if let Some(root) = &self.project_root {
            match storage::discover(root) {
                Ok(files) => self.files = files,
                Err(e) => self.error = Some(e),
            }
        }
        if let Some(root) = self
            .document
            .as_ref()
            .map(|d| d.root.clone())
            .or_else(|| self.project_root.clone())
        {
            self.reload_assets(&root);
        }
    }
    fn snapshot(&self) -> Option<Snapshot> {
        self.document.as_ref().map(|d| Snapshot {
            nodes: d.nodes.clone(),
            library: d.library.clone(),
        })
    }
    fn record(&mut self, before: Snapshot) {
        self.undo.push(before);
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    fn restore(&mut self, snapshot: Snapshot) {
        if let Some(doc) = &mut self.document {
            doc.nodes = snapshot.nodes;
            doc.library = snapshot.library;
        }
        self.play = None;
        self.rename_selection.clear();
        self.last_edit = -10.0;
    }
    fn undo(&mut self) {
        if let (Some(before), Some(now)) = (self.undo.pop(), self.snapshot()) {
            self.redo.push(now);
            self.restore(before);
        }
    }
    fn redo(&mut self) {
        if let (Some(after), Some(now)) = (self.redo.pop(), self.snapshot()) {
            self.undo.push(now);
            self.restore(after);
        }
    }
    fn request(&mut self, action: Pending) {
        if self.dirty() {
            self.pending = Some(action);
        } else {
            self.perform(action);
        }
    }
    fn perform(&mut self, action: Pending) {
        let result = match action {
            Pending::Load(root, path) => Document::load(&root, &path),
            Pending::New(root, name) => Document::create(&root, &name),
        };
        match result {
            Ok(doc) => {
                self.reload_assets(&doc.root);
                self.document = Some(doc);
                self.undo.clear();
                self.redo.clear();
                self.selected = 0;
                self.character = 0;
                self.play = None;
                self.events.clear();
                self.play_history.clear();
                self.error = None;
                self.status.clear();
                self.rename_selection.clear();
                self.textures.clear();
                self.pan = egui::vec2(30.0, 30.0);
                self.zoom = 1.0;
                self.last_edit = -10.0;
            }
            Err(e) => self.error = Some(e),
        }
    }
    pub fn shortcuts(&mut self, ctx: &Context) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save();
        }
        // Let focused text fields keep their own text undo history.
        if !ctx.wants_keyboard_input() {
            if ctx.input_mut(|i| {
                i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                )
            }) {
                self.redo();
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
                self.undo();
            }
        }
    }
    pub fn draw(&mut self, ctx: &Context, root: Option<&Path>) {
        if self.project_root.as_deref() != root {
            self.project_root = root.map(Path::to_path_buf);
            self.refresh();
            if !self.dirty() {
                if let (Some(root), Some(path)) = (root, self.files.first()) {
                    self.perform(Pending::Load(root.into(), path.clone()));
                } else {
                    self.document = None;
                }
            }
        }
        egui::TopBottomPanel::top("narrative_tools").frame(theme::panel_frame()).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("NARRATIVE").strong().color(theme::ACCENT));
                for (mode, label) in [(Mode::Story, "Story"), (Mode::Graph, "Graph"), (Mode::Characters, "Characters"), (Mode::Dependencies, "Dependencies"), (Mode::Preview, "Playthrough")] {
                    ui.selectable_value(&mut self.mode, mode, label);
                }
                ui.separator();
                if ui.add_enabled(root.is_some(), egui::Button::new("New story")).clicked() { self.creating = true; }
                if ui.button("Refresh").clicked() { self.refresh(); self.textures.clear(); }
                if ui.add_enabled(self.document.is_some(), egui::Button::new("Save narrative")).clicked() { self.save(); }
                if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Undo")).clicked() { self.undo(); }
                if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Redo")).clicked() { self.redo(); }
            });
            ui.horizontal_wrapped(|ui| {
                let title = self.document.as_ref().map(|d| d.path.file_name().unwrap_or_default().to_string_lossy().into_owned()).unwrap_or_else(|| "Choose or create a story".into());
                if let Some(path) = crate::editor::widgets::searchable_action(ui, "narrative_file", &title, &self.files.iter().map(|path| (path.clone(), path.file_name().unwrap_or_default().to_string_lossy().into_owned())).collect::<Vec<_>>()) {
                    if let Some(root) = root { self.request(Pending::Load(root.into(), path)); }
                }
                if self.dirty() { ui.colored_label(theme::AMBER, "Unsaved narrative"); }
                if let Some(doc) = &self.document {
                    if ui.small_button("Reload").clicked() { self.request(Pending::Load(doc.root.clone(), doc.path.clone())); }
                }
                if ui.small_button("Export Rust runtime").on_hover_text("Write src/narrative_runtime.rs for your game. See docs/narrative-editor.md for integration.").clicked() {
                    if let Some(doc) = &self.document {
                        match storage::export_runtime(&doc.root) {
                            Ok(path) => self.status = format!("Exported {}. Add mod narrative_runtime; to your game; see docs/narrative-editor.md.", path.display()),
                            Err(e) => self.error = Some(e),
                        }
                    }
                }
                if !self.status.is_empty() { ui.small(&self.status); }
            });
            if let Some(error) = &self.error { ui.colored_label(egui::Color32::LIGHT_RED, error); }
            if self.document.as_ref().is_some_and(|d| Some(d.root.as_path()) != root) {
                ui.colored_label(theme::AMBER, "Draft belongs to the previous project. Save it here, then choose a story from this project.");
            }
        });
        self.dialogs(ctx, root);
        let Some(mut doc) = self.document.take() else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_space(60.0);
                ui.heading("Every story starts with a choice.");
                ui.label("Create a story, or open a project with JSON stories in src/resources/story.");
                ui.label("Write in Story, build your cast in Characters, and try your story in Playthrough.");
            });
            return;
        };
        let before = Snapshot {
            nodes: doc.nodes.clone(),
            library: doc.library.clone(),
        };
        self.selected = self.selected.min(doc.nodes.len().saturating_sub(1));
        self.character = self
            .character
            .min(doc.library.characters.len().saturating_sub(1));
        egui::SidePanel::left("narrative_outline")
            .default_width(205.0)
            .width_range(170.0..=300.0)
            .frame(theme::panel_frame())
            .show(ctx, |ui| {
                theme::panel_heading(
                    ui,
                    if self.mode == Mode::Characters {
                        "Cast"
                    } else {
                        "Story outline"
                    },
                    "",
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Search text, IDs, speakers…")
                        .desired_width(f32::INFINITY),
                );
                if self.mode == Mode::Characters {
                    if ui.button("+ Character").clicked() {
                        let id = unique_id(
                            "character",
                            doc.library.characters.iter().map(|c| c.id.as_str()),
                        );
                        doc.library
                            .characters
                            .push(Character::new(id, "New character".into()));
                        self.character = doc.library.characters.len() - 1;
                    }
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, c) in doc.library.characters.iter().enumerate() {
                            if format!("{} {}", c.name, c.id)
                                .to_lowercase()
                                .contains(&self.search.to_lowercase())
                                && ui
                                    .selectable_label(self.character == i, &c.name)
                                    .on_hover_text(&c.id)
                                    .clicked()
                            {
                                self.character = i;
                            }
                        }
                    });
                } else {
                    if ui.button("+ Story step").clicked() {
                        let id = unique_id("step", doc.nodes.iter().map(|n| n.id.as_str()));
                        doc.nodes.push(StoryNode {
                            id,
                            text: "Write this moment…".into(),
                            ..Default::default()
                        });
                        self.selected = doc.nodes.len() - 1;
                        self.mode = Mode::Story;
                    }
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, n) in doc.nodes.iter().enumerate() {
                            if format!("{} {} {}", n.id, n.speaker, n.text)
                                .to_lowercase()
                                .contains(&self.search.to_lowercase())
                            {
                                let label =
                                    format!("{}{}", if i == 0 { "▶ " } else { "" }, step_label(n));
                                if ui
                                    .selectable_label(self.selected == i, label)
                                    .on_hover_text(&n.text)
                                    .clicked()
                                {
                                    self.selected = i;
                                    self.rename_selection.clear();
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "{} · {} choices",
                                        n.id,
                                        n.choices.len()
                                    ))
                                    .small()
                                    .color(theme::MUTED),
                                );
                            }
                        }
                    });
                }
            });
        match self.mode {
            Mode::Story => {
                egui::CentralPanel::default()
                    .frame(theme::panel_frame())
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical()
                            .show(ui, |ui| inspector::story(ui, self, &mut doc));
                    });
            }
            Mode::Graph => {
                egui::SidePanel::right("narrative_inspector")
                    .default_width(355.0)
                    .width_range(290.0..=600.0)
                    .resizable(true)
                    .frame(theme::panel_frame())
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical()
                            .show(ui, |ui| inspector::story(ui, self, &mut doc));
                    });
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(theme::BACKGROUND))
                    .show(ctx, |ui| graph::draw(ui, self, &mut doc));
            }
            Mode::Characters => {
                egui::CentralPanel::default()
                    .frame(theme::panel_frame())
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical()
                            .show(ui, |ui| characters::draw(ui, self, &mut doc));
                    });
            }
            Mode::Dependencies => {
                egui::CentralPanel::default()
                    .frame(theme::panel_frame())
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical()
                            .show(ui, |ui| inspector::dependencies(ui, self, &doc));
                    });
            }
            Mode::Preview => {
                egui::CentralPanel::default()
                    .frame(theme::panel_frame())
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| preview::draw(ui, self, &doc));
                    });
            }
        }
        let changed = before.nodes != doc.nodes || before.library != doc.library;
        self.document = Some(doc);
        if changed {
            let now = ctx.input(|i| i.time);
            if now - self.last_edit > 0.7 || self.undo.is_empty() {
                self.record(before);
            } else {
                self.redo.clear();
            }
            self.last_edit = now;
            self.play = None;
        }
    }
    fn dialogs(&mut self, ctx: &Context, root: Option<&Path>) {
        if self.creating {
            egui::Window::new("New story")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Filename (without .json)");
                    ui.text_edit_singleline(&mut self.new_name);
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            if let Some(root) = root {
                                self.request(Pending::New(
                                    root.into(),
                                    self.new_name.trim().into(),
                                ));
                            }
                            self.creating = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.creating = false;
                        }
                    });
                });
        }
        if self.pending.is_some() {
            egui::Window::new("Keep your narrative changes?")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("This story or its shared character library has unsaved changes.");
                    ui.horizontal(|ui| {
                        if ui.button("Save and continue").clicked() {
                            self.save();
                            if !self.dirty() {
                                if let Some(action) = self.pending.take() {
                                    self.perform(action);
                                }
                            }
                        }
                        if ui.button("Discard and continue").clicked() {
                            if let Some(action) = self.pending.take() {
                                self.perform(action);
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.pending = None;
                        }
                    });
                });
        }
    }
}

/// Creates a destination and its incoming choice together, so writing never
/// leaves a dangling link. Existing routes and requirements remain untouched.
fn add_connected_step(
    nodes: &mut Vec<StoryNode>,
    source: usize,
    choice_text: &str,
) -> Option<usize> {
    if source >= nodes.len() {
        return None;
    }
    let id = unique_id("step", nodes.iter().map(|n| n.id.as_str()));
    nodes[source].choices.push(crate::narrative::Choice {
        text: choice_text.into(),
        target: id.clone(),
        ..Default::default()
    });
    nodes.push(StoryNode {
        id,
        ..Default::default()
    });
    Some(nodes.len() - 1)
}

fn step_label(node: &StoryNode) -> String {
    let text = if node.text.trim().is_empty() {
        "Unwritten moment"
    } else {
        &node.text
    };
    let preview = inspector::excerpt(text, 64);
    if node.speaker.is_empty() {
        preview
    } else {
        format!("{}: {preview}", node.speaker)
    }
}

fn unique_id<'a>(prefix: &str, existing: impl Iterator<Item = &'a str>) -> String {
    let ids: Vec<_> = existing.collect();
    (1..)
        .map(|n| format!("{prefix}_{n}"))
        .find(|id| !ids.contains(&id.as_str()))
        .unwrap()
}

fn portrait(
    ui: &mut egui::Ui,
    editor: &mut NarrativeEditor,
    root: &Path,
    relative: &str,
    size: f32,
) {
    let key = format!("{}:{relative}", root.display());
    let texture = editor.textures.entry(key.clone()).or_insert_with(|| {
        let path = storage::resource_path(root, relative)?;
        let mut reader = image::io::Reader::open(path)
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
            .map_err(|e| e.to_string())?
            .thumbnail(512, 512)
            .to_rgba8();
        let color = egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        );
        Ok(ui
            .ctx()
            .load_texture(key, color, egui::TextureOptions::LINEAR))
    });
    match texture {
        Ok(texture) => {
            let dimensions = texture.size_vec2();
            let scale = (size / dimensions.x).min(size / dimensions.y);
            ui.image((texture.id(), dimensions * scale));
        }
        Err(e) => {
            ui.colored_label(theme::AMBER, format!("Portrait unavailable: {e}"));
        }
    }
}

#[cfg(test)]
mod tests;
