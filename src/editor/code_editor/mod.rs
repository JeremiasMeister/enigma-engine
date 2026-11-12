mod buffer;
mod external;
mod highlight;

use crate::editor::{
    state::{EditorRoot, ProjectState},
    theme,
};
use crate::project::code_files::{self, FileEntry, SaveError, TextDocument, MAX_TEXT_BYTES};
use egui::{Context, RichText, TextureHandle};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use uuid::Uuid;

#[derive(Default)]
struct Tree {
    children: BTreeMap<OsString, Tree>,
    file: Option<FileEntry>,
}
impl Tree {
    fn from_files(files: &[FileEntry]) -> Self {
        let mut tree = Self::default();
        for file in files {
            let mut branch = &mut tree;
            for component in &file.relative {
                branch = branch.children.entry(component.into()).or_default();
            }
            branch.file = Some(file.clone());
        }
        tree
    }
}

enum Content {
    Text(TextDocument),
    Image(TextureHandle, [usize; 2]),
    Binary(String),
}
struct Tab {
    id: Uuid,
    project_name: String,
    root: PathBuf,
    relative: PathBuf,
    content: Content,
    read_only: bool,
    generated: bool,
    highlight: highlight::Highlight,
    query: String,
    match_index: usize,
    draft_error: Option<String>,
    input_limited: bool,
}
impl Tab {
    fn dirty(&self) -> bool {
        matches!(&self.content,Content::Text(document) if document.dirty())
    }
    fn title(&self) -> String {
        format!(
            "{}{}",
            self.relative
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            if self.dirty() { " *" } else { "" }
        )
    }
}
#[derive(Clone, Copy)]
enum PendingKind {
    Close,
    Reload,
    Conflict,
}
#[derive(Clone, Copy)]
struct Pending {
    id: Uuid,
    kind: PendingKind,
}

#[derive(Default)]
pub struct CodeEditorState {
    tabs: Vec<Tab>,
    active: Option<usize>,
    tree: Tree,
    files: Vec<FileEntry>,
    tree_root: Option<PathBuf>,
    scan: Option<Receiver<Result<Vec<FileEntry>, String>>>,
    launch: Option<Receiver<Result<String, String>>>,
    editor: external::Editor,
    search: String,
    pending: Option<Pending>,
    pub status: String,
    pub error: Option<String>,
}

impl CodeEditorState {
    pub fn shortcuts(&mut self, ctx: &Context) {
        if ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::S,
            )
        }) {
            let _ = self.save_all();
        } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save_active();
        }
    }
    pub fn dirty_count(&self) -> usize {
        self.tabs.iter().filter(|tab| tab.dirty()).count()
    }
    pub fn poll(&mut self, ctx: &Context) -> Option<String> {
        if let Some(rx) = &self.scan {
            if let Ok(result) = rx.try_recv() {
                self.scan = None;
                match result {
                    Ok(files) => {
                        self.tree = Tree::from_files(&files);
                        self.files = files;
                    }
                    Err(error) => self.error = Some(error),
                }
            }
        }
        let mut launch_error = None;
        if let Some(rx) = &self.launch {
            if let Ok(result) = rx.try_recv() {
                self.launch = None;
                match result {
                    Ok(message) => self.status = message,
                    Err(error) => {
                        self.error = Some(error.clone());
                        launch_error = Some(error);
                    }
                }
            }
        }
        if self.scan.is_some() || self.launch.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
        launch_error
    }
    fn refresh(&mut self, project: &ProjectState) {
        let root = PathBuf::from(&project.root_path);
        self.tree_root = Some(root.clone());
        self.files.clear();
        self.tree = Tree::default();
        let (tx, rx) = mpsc::channel();
        self.scan = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(code_files::list(&root));
        });
    }
    pub fn invalidate_tree(&mut self) {
        self.tree_root = None;
    }
    pub fn open_project(&mut self, project: &ProjectState) {
        self.launch_path(PathBuf::from(&project.root_path), false);
    }
    fn launch_path(&mut self, path: PathBuf, reveal: bool) {
        if self.launch.is_some() {
            return;
        }
        let editor = self.editor;
        let (tx, rx) = mpsc::channel();
        self.launch = Some(rx);
        std::thread::spawn(move || {
            let result = if reveal {
                external::reveal(&path)
            } else {
                external::open(editor, &path)
            };
            let _ = tx.send(result);
        });
    }
    fn open(&mut self, ctx: &Context, project: &ProjectState, relative: &Path) {
        let root = match Path::new(&project.root_path).canonicalize() {
            Ok(root) => root,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| tab.root == root && tab.relative == relative)
        {
            self.active = Some(index);
            return;
        }
        if self.tabs.len() >= 24 {
            self.error = Some("Close a tab before opening more than 24 files.".into());
            return;
        }
        match load_tab(ctx, &root, relative, &project.name) {
            Ok(tab) => {
                self.tabs.push(tab);
                self.active = Some(self.tabs.len() - 1);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
    pub fn save_active(&mut self) {
        if let Some(index) = self.active {
            let _ = self.save_tab(index, false);
        }
    }
    fn save_tab(&mut self, index: usize, overwrite: bool) -> Result<(), String> {
        let Some(tab) = self.tabs.get_mut(index) else {
            return Ok(());
        };
        if tab.read_only && !managed(&tab.relative) {
            if let Ok(path) = code_files::checked_path(&tab.root, &tab.relative, false) {
                if let Ok(metadata) = std::fs::metadata(path) {
                    tab.read_only = metadata.permissions().readonly();
                }
            }
        }
        if tab.read_only {
            if tab.dirty() {
                let message = format!("{} is read-only. Your unsaved draft is still available; make the file writable and save again, or explicitly discard the draft.", tab.relative.display());
                self.error = Some(message.clone());
                return Err(message);
            }
            return Ok(());
        }
        if let Content::Text(document) = &mut tab.content {
            if !document.dirty() {
                return Ok(());
            }
            match document.save(overwrite) {
                Ok(()) => {
                    tab.draft_error = None;
                    self.error = None;
                    self.status =
                        format!("Saved {} · {}", tab.project_name, tab.relative.display());
                }
                Err(error) => {
                    if error == SaveError::Conflict {
                        self.pending = Some(Pending {
                            id: tab.id,
                            kind: PendingKind::Conflict,
                        });
                    }
                    let message = format!("{}: {error}", tab.relative.display());
                    self.error = Some(message.clone());
                    return Err(message);
                }
            }
        }
        Ok(())
    }
    fn save_all(&mut self) -> Result<(), String> {
        for index in 0..self.tabs.len() {
            self.save_tab(index, false)?;
        }
        Ok(())
    }
    pub fn save_project(&mut self, root: &Path) -> Result<(), String> {
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        for index in 0..self.tabs.len() {
            if self.tabs[index].root == root {
                self.save_tab(index, false)?;
            }
        }
        Ok(())
    }
    fn request(&mut self, index: usize, kind: PendingKind, ctx: &Context) {
        if self.tabs[index].dirty() {
            self.pending = Some(Pending {
                id: self.tabs[index].id,
                kind,
            });
        } else {
            self.finish(index, kind, ctx);
        }
    }
    fn finish(&mut self, index: usize, kind: PendingKind, ctx: &Context) {
        let tab = &self.tabs[index];
        match kind {
            PendingKind::Close => {
                if let Content::Text(document) = &tab.content {
                    if let Err(error) = document.remove_draft() {
                        self.error = Some(error);
                        return;
                    }
                }
                self.tabs.remove(index);
                self.active = if self.tabs.is_empty() {
                    None
                } else {
                    Some(
                        self.active
                            .unwrap_or(0)
                            .saturating_sub(usize::from(index < self.active.unwrap_or(0)))
                            .min(self.tabs.len() - 1),
                    )
                };
            }
            PendingKind::Reload => {
                // Read the replacement before removing the current draft. A failed
                // read keeps both the buffer and its recovery copy intact.
                let result = reload_tab(ctx, tab);
                match result {
                    Ok(replacement) => {
                        self.tabs[index] = replacement;
                        self.error = None;
                        self.status = "Reloaded from disk.".into();
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            PendingKind::Conflict => {}
        }
    }
}

fn managed(relative: &Path) -> bool {
    relative == Path::new("enigma_project.json")
        || (relative.starts_with("src/resources/scenes")
            && relative.extension().is_some_and(|e| e == "json"))
}

fn load_tab(
    ctx: &Context,
    root: &Path,
    relative: &Path,
    project_name: &str,
) -> Result<Tab, String> {
    load_tab_version(ctx, root, relative, project_name, true)
}

fn load_tab_version(
    ctx: &Context,
    root: &Path,
    relative: &Path,
    project_name: &str,
    recover: bool,
) -> Result<Tab, String> {
    let path = code_files::checked_path(root, relative, false)?;
    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("Select a regular file.".into());
    }
    let extension = relative
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    let content = if matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tga"
    ) {
        match code_files::read(root, relative, 64 * 1024 * 1024)
            .and_then(|bytes| crate::project::ui_assets::decode(&bytes))
        {
            Ok(image) => {
                let max = ctx.input(|i| i.max_texture_side);
                if image.size.iter().any(|s| *s > max) {
                    Content::Binary(format!("Image exceeds this GPU's {max}px texture limit."))
                } else {
                    let pixels = egui::ColorImage::from_rgba_unmultiplied(image.size, &image.rgba);
                    Content::Image(
                        ctx.load_texture(
                            path.to_string_lossy(),
                            pixels,
                            egui::TextureOptions::LINEAR,
                        ),
                        image.size,
                    )
                }
            }
            Err(error) => Content::Binary(error),
        }
    } else {
        match if recover {
            TextDocument::open(root, relative)
        } else {
            TextDocument::open_disk(root, relative)
        } {
            Ok(document) => Content::Text(document),
            Err(error) => Content::Binary(format!("{} bytes · {error}", metadata.len())),
        }
    };
    let generated = matches!(&content,Content::Text(document) if document.text.starts_with("// Generated by Enigma UI Editor."));
    Ok(Tab {
        id: Uuid::new_v4(),
        project_name: project_name.into(),
        root: root.into(),
        relative: relative.into(),
        content,
        read_only: managed(relative) || metadata.permissions().readonly(),
        generated,
        highlight: Default::default(),
        query: String::new(),
        match_index: 0,
        draft_error: None,
        input_limited: false,
    })
}

fn reload_tab(ctx: &Context, tab: &Tab) -> Result<Tab, String> {
    // Obtain the replacement first, so a failed reload leaves the draft intact.
    code_files::read(&tab.root, &tab.relative, 64 * 1024 * 1024)?;
    let replacement = load_tab_version(ctx, &tab.root, &tab.relative, &tab.project_name, false)?;
    if let Content::Text(document) = &tab.content {
        document.remove_draft()?;
    }
    Ok(replacement)
}

pub fn draw(ctx: &Context, root: &mut EditorRoot) {
    let state = &mut root.editor.code_editor;
    let Some(project) = root.project.as_ref() else {
        egui::CentralPanel::default()
            .frame(theme::panel_frame())
            .show(ctx, |ui| {
                ui.heading("Project code");
                ui.label("Open a project to browse its Rust source and assets.");
            });
        return;
    };
    if state.tree_root.as_deref() != Some(Path::new(&project.root_path)) {
        state.refresh(project);
    }
    let mut open = None;
    let mut refresh = false;
    let mut external = false;
    let mut reveal = false;
    egui::SidePanel::left("project_files")
        .default_width(255.0)
        .min_width(190.0)
        .frame(theme::panel_frame())
        .show(ctx, |ui| {
            theme::panel_heading(ui, "Project files", &project.name);
            crate::editor::widgets::search_field(ui, &mut state.search, "Find a file…");
            ui.horizontal(|ui| {
                refresh = ui.small_button("Refresh").clicked();
                if state.scan.is_some() {
                    ui.spinner();
                } else {
                    ui.weak(format!("{} files", state.files.len()));
                }
            });
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let selected = state
                        .active
                        .and_then(|i| state.tabs.get(i))
                        .filter(|tab| {
                            tab.root
                                == Path::new(&project.root_path)
                                    .canonicalize()
                                    .unwrap_or_default()
                        })
                        .map(|tab| tab.relative.as_path());
                    if state.search.trim().is_empty() {
                        draw_tree(ui, &state.tree, Path::new(""), selected, &mut open);
                    } else {
                        let search = state.search.to_lowercase();
                        for file in state
                            .files
                            .iter()
                            .filter(|file| {
                                file.relative
                                    .to_string_lossy()
                                    .to_lowercase()
                                    .contains(&search)
                            })
                            .take(500)
                        {
                            if ui
                                .selectable_label(
                                    selected == Some(file.relative.as_path()),
                                    file.relative.to_string_lossy(),
                                )
                                .on_hover_text(format!("{} bytes", file.size))
                                .clicked()
                            {
                                open = Some(file.relative.clone());
                            }
                        }
                    }
                });
        });
    if refresh {
        state.refresh(project);
    }
    if let Some(path) = open {
        state.open(ctx, project, &path);
    }
    egui::TopBottomPanel::top("code_actions")
        .frame(theme::panel_frame())
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Source").strong().color(theme::ACCENT));
                if ui
                    .add_enabled(state.active.is_some(), egui::Button::new("Save file"))
                    .clicked()
                {
                    state.save_active();
                }
                if ui
                    .add_enabled(state.dirty_count() > 0, egui::Button::new("Save all"))
                    .clicked()
                {
                    let _ = state.save_all();
                }
                if ui
                    .add_enabled(state.active.is_some(), egui::Button::new("Reload"))
                    .clicked()
                {
                    state.request(state.active.unwrap(), PendingKind::Reload, ctx);
                }
                ui.separator();
                egui::ComboBox::from_id_source("external_code_editor")
                    .selected_text(state.editor.label())
                    .show_ui(ui, |ui| {
                        for editor in external::Editor::ALL {
                            ui.selectable_value(&mut state.editor, editor, editor.label());
                        }
                    });
                external = ui
                    .add_enabled(
                        state.launch.is_none(),
                        egui::Button::new("Open Rust project"),
                    )
                    .clicked();
                reveal = ui.button("Reveal folder").clicked();
                if state.launch.is_some() {
                    ui.spinner();
                }
            });
        });
    if external {
        state.open_project(project);
    }
    if reveal {
        state.launch_path(PathBuf::from(&project.root_path), true);
    }
    egui::TopBottomPanel::bottom("code_feedback")
        .frame(theme::panel_frame())
        .show(ctx, |ui| {
            if let Some(error) = state.error.clone() {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(theme::AMBER, error);
                    if ui.small_button("Dismiss").clicked() {
                        state.error = None;
                    }
                });
            }
            ui.small(if state.status.is_empty() {
                "Ctrl/Cmd+S Save file · Ctrl/Cmd+Shift+S Save all · Ctrl/Cmd+F Find"
            } else {
                &state.status
            });
        });
    let mut close = None;
    let mut open_file = None;
    let mut reveal_file = None;
    egui::CentralPanel::default().frame(theme::panel_frame()).show(ctx,|ui|{
        egui::ScrollArea::horizontal().id_source("code_tabs").auto_shrink([false,true]).show(ui,|ui|{ui.horizontal(|ui|{
            for (index,tab) in state.tabs.iter().enumerate(){ui.push_id(tab.id,|ui|{
                if ui.selectable_label(state.active==Some(index),tab.title()).on_hover_text(format!("{} · {}",tab.project_name,tab.relative.display())).clicked(){state.active=Some(index);}
                if ui.small_button("×").on_hover_text("Close file").clicked(){close=Some(index);}
            });}
        });});
        ui.separator();
        if let Some(index)=state.active {
            let tab=&mut state.tabs[index];
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new(format!("{} / {}",tab.project_name,tab.relative.display())).color(theme::MUTED));
                if ui.small_button("Open file externally").clicked(){open_file=Some(tab.root.join(&tab.relative));}
                if ui.small_button("Show in folder").clicked(){reveal_file=Some(tab.root.join(&tab.relative));}
            });
            if tab.read_only{ui.colored_label(theme::AMBER,if managed(&tab.relative){"Enigma manages this file. Use the scene/UI tools to change it."}else{"This file is read-only."});}
            if tab.generated{ui.colored_label(theme::AMBER,"Generated UI file: Generate Rust replaces edits here. Keep game behavior in your own source files.");}
            show_content(ui,tab,ctx);
        }else{
            ui.add_space(45.0);ui.heading("Your game's source, close at hand");
            ui.label("Choose a Rust, config or text file to edit, or select an image to preview it.");
            ui.add_space(10.0);ui.weak("Open Rust project launches your full IDE for completion, refactoring and debugging.");
            ui.weak("Dirty text is recovered from local drafts when you reopen a file. Save writes the actual source.");
        }
    });
    if let Some(index) = close {
        state.request(index, PendingKind::Close, ctx);
    }
    if let Some(path) = open_file {
        state.launch_path(path, false);
    }
    if let Some(path) = reveal_file {
        state.launch_path(path, true);
    }
    pending_dialog(ctx, state);
}

fn draw_tree(
    ui: &mut egui::Ui,
    tree: &Tree,
    prefix: &Path,
    selected: Option<&Path>,
    open: &mut Option<PathBuf>,
) {
    for (name, branch) in tree.children.iter().filter(|(_, t)| t.file.is_none()) {
        let path = prefix.join(name);
        egui::CollapsingHeader::new(name.to_string_lossy())
            .id_source(&path)
            .default_open(path == Path::new("src"))
            .show(ui, |ui| draw_tree(ui, branch, &path, selected, open));
    }
    for (name, branch) in tree.children.iter().filter(|(_, t)| t.file.is_some()) {
        let file = branch.file.as_ref().unwrap();
        if ui
            .selectable_label(
                selected == Some(file.relative.as_path()),
                name.to_string_lossy(),
            )
            .on_hover_text(format!("{} · {} bytes", file.relative.display(), file.size))
            .clicked()
        {
            *open = Some(file.relative.clone());
        }
    }
}

fn show_content(ui: &mut egui::Ui, tab: &mut Tab, ctx: &Context) {
    match &mut tab.content {
        Content::Text(document) => {
            if document.recovered {
                ui.colored_label(theme::AMBER,"Recovered an unsaved draft. Save to write it, or Reload to use the disk version.");
            }
            let mut find = false;
            let mut direction = 0;
            ui.push_id(tab.id, |ui| {
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut tab.query)
                            .id_source(("code_find", tab.id))
                            .hint_text("Find in file…")
                            .desired_width(220.0),
                    );
                    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
                        response.request_focus();
                    }
                    if response.changed() {
                        tab.match_index = 0;
                        find = true;
                    }
                    if ui.small_button("Previous").clicked() {
                        direction = -1;
                        find = true;
                    }
                    if ui.small_button("Next").clicked() {
                        direction = 1;
                        find = true;
                    }
                    if !tab.query.is_empty() {
                        let count = document.text.match_indices(&tab.query).take(10000).count();
                        ui.weak(format!("{count} matches"));
                    }
                });
            });
            let selection = if find && !tab.query.is_empty() {
                let matches: Vec<_> = document
                    .text
                    .match_indices(&tab.query)
                    .take(10000)
                    .map(|(i, _)| i)
                    .collect();
                if matches.is_empty() {
                    None
                } else {
                    tab.match_index = if direction < 0 {
                        (tab.match_index + matches.len() - 1) % matches.len()
                    } else if direction > 0 {
                        (tab.match_index + 1) % matches.len()
                    } else {
                        tab.match_index.min(matches.len() - 1)
                    };
                    let byte = matches[tab.match_index];
                    let start = document.text[..byte].chars().count();
                    Some((start, start + tab.query.chars().count()))
                }
            } else {
                None
            };
            let mut changed = false;
            if tab.input_limited {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(theme::AMBER, "Input was limited to keep this file within 1 MiB of text. Open it externally for larger edits.");
                    if ui.small_button("Dismiss").clicked() { tab.input_limited = false; }
                });
            }
            if let Some(error) = &tab.draft_error {
                ui.colored_label(
                    theme::AMBER,
                    format!("Draft could not be backed up: {error}. Save the file before closing."),
                );
            }
            egui::ScrollArea::both()
                .id_source(("code_scroll", tab.id))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        let font = egui::FontId::monospace(14.0);
                        let height = ui.fonts(|f| f.row_height(&font));
                        let lines = document.text.split('\n').count();
                        let width = 20.0 + (lines.to_string().len() as f32) * 9.0;
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(width, height * lines as f32),
                            egui::Sense::hover(),
                        );
                        let first =
                            ((ui.clip_rect().top() - rect.top()) / height).max(0.0) as usize;
                        let last = (((ui.clip_rect().bottom() - rect.top()) / height)
                            .ceil()
                            .max(0.0) as usize)
                            .min(lines);
                        for line in first..last {
                            ui.painter().text(
                                egui::pos2(rect.right() - 8.0, rect.top() + line as f32 * height),
                                egui::Align2::RIGHT_TOP,
                                (line + 1).to_string(),
                                font.clone(),
                                theme::MUTED,
                            );
                        }
                        let rust = tab.relative.extension().is_some_and(|e| e == "rs");
                        let mut layouter = |ui: &egui::Ui, text: &str, _width: f32| {
                            let job = tab.highlight.layout(text, rust);
                            ui.fonts(|f| f.layout_job(job))
                        };
                        let (capacity, crlf) = document.editing_limits();
                        let mut buffer = buffer::BoundedText {
                            text: &mut document.text,
                            capacity,
                            crlf,
                            limited: &mut tab.input_limited,
                        };
                        let mut output = egui::TextEdit::multiline(&mut buffer)
                            .id_source(("code_text", tab.id))
                            .code_editor()
                            .font(font)
                            .desired_width(f32::INFINITY)
                            .desired_rows(lines.max(24))
                            .frame(false)
                            .margin(egui::Vec2::ZERO)
                            .interactive(!tab.read_only)
                            .char_limit(MAX_TEXT_BYTES)
                            .layouter(&mut layouter)
                            .show(ui);
                        changed = output.response.changed();
                        if let Some((start, end)) = selection {
                            use egui::text::{CCursor, CCursorRange};
                            output.state.set_ccursor_range(Some(CCursorRange::two(
                                CCursor::new(start),
                                CCursor::new(end),
                            )));
                            output.state.store(ctx, output.response.id);
                            if direction != 0 {
                                output.response.request_focus();
                            }
                            let cursor = output.galley.from_ccursor(CCursor::new(start));
                            ui.scroll_to_rect(
                                output
                                    .galley
                                    .pos_from_cursor(&cursor)
                                    .translate(output.text_draw_pos.to_vec2()),
                                Some(egui::Align::Center),
                            );
                        }
                    });
                });
            if changed {
                tab.draft_error = document.store_draft().err();
            }
        }
        Content::Image(texture, size) => {
            ui.label(format!("{} × {} pixels", size[0], size[1]));
            let available = ui.available_size().max(egui::vec2(1.0, 1.0));
            let scale = (available.x / size[0] as f32)
                .min(available.y / size[1] as f32)
                .min(1.0);
            ui.image((
                texture.id(),
                egui::vec2(size[0] as f32, size[1] as f32) * scale,
            ));
        }
        Content::Binary(message) => {
            ui.add_space(30.0);
            ui.heading("File preview");
            ui.label(message.as_str());
            ui.weak("Open the file externally to inspect this format or larger files.");
        }
    }
}

fn pending_dialog(ctx: &Context, state: &mut CodeEditorState) {
    let Some(pending) = state.pending else {
        return;
    };
    let Some(index) = state.tabs.iter().position(|tab| tab.id == pending.id) else {
        state.pending = None;
        return;
    };
    let title = match pending.kind {
        PendingKind::Conflict => "File changed on disk",
        _ => "Unsaved file",
    };
    let mut action = 0;
    egui::Window::new(title).id(egui::Id::new("code_file_confirmation")).anchor(egui::Align2::CENTER_CENTER,egui::Vec2::ZERO).collapsible(false).resizable(false).show(ctx,|ui|{
        ui.label(format!("{} / {}",state.tabs[index].project_name,state.tabs[index].relative.display()));
        ui.label(match pending.kind{PendingKind::Conflict=>"The disk version differs from the version you opened. Your buffer and recovery draft are still intact.",_=>"Save your changes, discard them, or keep editing."});
        ui.horizontal(|ui|{
            if ui.button(if matches!(pending.kind,PendingKind::Conflict){"Overwrite disk"}else{"Save"}).clicked(){action=1;}
            if ui.button(if matches!(pending.kind,PendingKind::Conflict){"Reload disk"}else{"Discard changes"}).clicked(){action=2;}
            if ui.button("Cancel").clicked(){action=3;}
        });
    });
    if action > 0 {
        state.pending = None;
    }
    match action {
        1 => {
            if state
                .save_tab(index, matches!(pending.kind, PendingKind::Conflict))
                .is_ok()
            {
                state.finish(index, pending.kind, ctx);
            }
        }
        2 => state.finish(
            index,
            if matches!(pending.kind, PendingKind::Conflict) {
                PendingKind::Reload
            } else {
                pending.kind
            },
            ctx,
        ),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_reload_reads_disk_and_failed_reload_preserves_the_draft() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("main.rs");
        std::fs::write(&path, "original").unwrap();
        let mut document = TextDocument::open(root.path(), Path::new("main.rs")).unwrap();
        document.text = "unsaved".into();
        document.store_draft().unwrap();
        let ctx = Context::default();
        let tab = load_tab(&ctx, root.path(), Path::new("main.rs"), "Example").unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(reload_tab(&ctx, &tab).is_err());
        std::fs::write(&path, "external").unwrap();
        assert!(
            TextDocument::open(root.path(), Path::new("main.rs"))
                .unwrap()
                .recovered
        );
        let reloaded = reload_tab(&ctx, &tab).unwrap();
        assert!(
            matches!(reloaded.content, Content::Text(document) if document.text == "external" && !document.recovered)
        );
        assert!(
            !TextDocument::open(root.path(), Path::new("main.rs"))
                .unwrap()
                .recovered
        );
    }

    #[test]
    fn saving_a_read_only_recovered_draft_keeps_the_tab_and_draft() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("main.rs");
        std::fs::write(&path, "original").unwrap();
        let mut document = TextDocument::open(root.path(), Path::new("main.rs")).unwrap();
        document.text = "unsaved".into();
        document.store_draft().unwrap();
        let ctx = Context::default();
        let permissions = std::fs::metadata(&path).unwrap().permissions();
        let mut readonly = permissions.clone();
        readonly.set_readonly(true);
        std::fs::set_permissions(&path, readonly).unwrap();
        let tab = load_tab(
            &ctx,
            &root.path().canonicalize().unwrap(),
            Path::new("main.rs"),
            "Example",
        )
        .unwrap();
        let mut state = CodeEditorState {
            tabs: vec![tab],
            active: Some(0),
            ..Default::default()
        };
        // The confirmation closes a tab only if save_tab succeeds.
        if state.save_tab(0, false).is_ok() {
            state.finish(0, PendingKind::Close, &ctx);
        }
        assert_eq!(state.tabs.len(), 1);
        assert!(state.save_project(root.path()).is_err());
        assert!(
            TextDocument::open(root.path(), Path::new("main.rs"))
                .unwrap()
                .recovered
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
        std::fs::set_permissions(&path, permissions).unwrap();
        state.save_active();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "unsaved");
    }

    #[test]
    fn failed_draft_warning_is_visible_inside_the_content_panel() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("main.rs"), "fn main() {}\n").unwrap();
        let ctx = Context::default();
        let mut tab = load_tab(&ctx, root.path(), Path::new("main.rs"), "Example").unwrap();
        tab.draft_error = Some("Permission denied".into());
        ctx.begin_frame(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 650.0),
            )),
            ..Default::default()
        });
        egui::CentralPanel::default().show(&ctx, |ui| show_content(ui, &mut tab, &ctx));
        let output = ctx.end_frame();
        assert!(output.shapes.iter().any(|shape| {
            if let egui::epaint::Shape::Text(text) = &shape.shape {
                return text
                    .galley
                    .text()
                    .starts_with("Draft could not be backed up")
                    && shape.clip_rect.contains(text.pos)
                    && text.pos.y < 600.0;
            }
            false
        }));
    }

    #[test]
    fn typing_in_the_source_widget_creates_a_draft_and_save_writes_source() {
        let root = tempfile::tempdir().unwrap();
        let original = "fn main() {}\n";
        let edited = "fn main() { println!(\"Hello, 世界\"); }\n";
        std::fs::write(root.path().join("main.rs"), original).unwrap();
        let ctx = Context::default();
        let mut tab = load_tab(&ctx, root.path(), Path::new("main.rs"), "Example").unwrap();
        let frame = |tab: &mut Tab, events: Vec<egui::Event>| {
            ctx.begin_frame(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 650.0),
                )),
                events,
                ..Default::default()
            });
            egui::CentralPanel::default().show(&ctx, |ui| show_content(ui, tab, &ctx));
            ctx.end_frame()
        };
        let output = frame(&mut tab, vec![]);
        let position = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::epaint::Shape::Text(text) = &shape.shape {
                    if text.galley.text() == original {
                        return Some(text.pos + egui::vec2(3.0, 6.0));
                    }
                }
                None
            })
            .expect("source text is visible");
        frame(
            &mut tab,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        frame(
            &mut tab,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        frame(
            &mut tab,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                },
                egui::Event::Text(edited.into()),
            ],
        );
        assert!(matches!(&tab.content, Content::Text(document) if document.text == edited));
        let copied = frame(
            &mut tab,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                },
                egui::Event::Copy,
            ],
        );
        assert_eq!(copied.platform_output.copied_text, edited);
        let cut = frame(&mut tab, vec![egui::Event::Cut]);
        assert_eq!(cut.platform_output.copied_text, edited);
        assert!(matches!(&tab.content, Content::Text(document) if document.text.is_empty()));
        frame(
            &mut tab,
            vec![egui::Event::Paste(copied.platform_output.copied_text)],
        );
        assert!(matches!(&tab.content, Content::Text(document) if document.text == edited));
        assert_eq!(
            std::fs::read_to_string(root.path().join("main.rs")).unwrap(),
            original
        );
        let recovered = TextDocument::open(root.path(), Path::new("main.rs")).unwrap();
        assert!(recovered.recovered);
        assert_eq!(recovered.text, edited);
        let mut state = CodeEditorState {
            tabs: vec![tab],
            active: Some(0),
            ..Default::default()
        };
        state.save_active();
        assert_eq!(
            std::fs::read_to_string(root.path().join("main.rs")).unwrap(),
            edited
        );
        assert!(!state.tabs[0].dirty());
    }

    #[test]
    fn dirty_tabs_save_to_their_own_project_and_survive_switching() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        for root in [a.path(), b.path()] {
            std::fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
        }
        let ctx = Context::default();
        ctx.begin_frame(Default::default());
        let mut state = CodeEditorState::default();
        let pa = ProjectState::new("A".into(), a.path().to_string_lossy().into());
        let pb = ProjectState::new("B".into(), b.path().to_string_lossy().into());
        state.open(&ctx, &pa, Path::new("main.rs"));
        if let Content::Text(document) = &mut state.tabs[0].content {
            document.text = "// A unsaved".into();
        }
        state.open(&ctx, &pb, Path::new("main.rs"));
        if let Content::Text(document) = &mut state.tabs[1].content {
            document.text = "// B edit".into();
        }
        state.save_project(b.path()).unwrap();
        assert!(state.tabs[0].dirty());
        assert!(!state.tabs[1].dirty());
        assert_eq!(
            std::fs::read_to_string(a.path().join("main.rs")).unwrap(),
            "fn main() {}\n"
        );
        state.active = Some(0);
        state.save_active();
        assert_eq!(
            std::fs::read_to_string(a.path().join("main.rs")).unwrap(),
            "// A unsaved"
        );
        let _ = ctx.end_frame();
    }
    #[test]
    fn managed_metadata_is_read_only_but_game_sources_are_editable() {
        assert!(managed(Path::new("enigma_project.json")));
        assert!(managed(Path::new("src/resources/scenes/main.json")));
        assert!(!managed(Path::new("src/main.rs")));
        assert!(!managed(Path::new("Cargo.toml")));
    }
    #[test]
    fn save_project_stops_on_conflict_and_keeps_the_buffer() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("main.rs"), "old").unwrap();
        let ctx = Context::default();
        ctx.begin_frame(Default::default());
        let mut state = CodeEditorState::default();
        let project = ProjectState::new("P".into(), root.path().to_string_lossy().into());
        state.open(&ctx, &project, Path::new("main.rs"));
        if let Content::Text(document) = &mut state.tabs[0].content {
            document.text = "mine".into();
        }
        std::fs::write(root.path().join("main.rs"), "external").unwrap();
        assert!(state.save_project(root.path()).is_err());
        assert!(state.tabs[0].dirty());
        assert!(state.pending.is_some());
        assert_eq!(
            std::fs::read_to_string(root.path().join("main.rs")).unwrap(),
            "external"
        );
        let _ = ctx.end_frame();
    }
}
