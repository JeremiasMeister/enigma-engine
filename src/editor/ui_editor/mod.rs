mod appearance;
mod canvas;
mod history;
mod inspector;

use crate::editor::{state::EditorRoot, theme};
use crate::project::ui_codegen;
use crate::ui_runtime::{UiData, UiItem, UiNode, UiScreen, UiTextures, WidgetKind};
use egui::{Context, RichText};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Design,
    Preview,
    Code,
}

pub struct UiEditorState {
    egui_id: egui::Id,
    pub active_screen: Option<String>,
    pub selected: Option<u64>,
    pub mode: Mode,
    pub snap: bool,
    pub zoom: f32,
    pub preview_data: UiData,
    pub sample_json: String,
    pub status: String,
    pub error: Option<String>,
    pub search: String,
    pub drag: Option<canvas::Drag>,
    history: history::History,
    cached_code: Option<(UiScreen, String)>,
    delete_screen: bool,
    selected_style: String,
    import_texture: bool,
    textures: UiTextures,
    texture_sources: Option<(String, std::collections::BTreeMap<String, String>)>,
    texture_error: Option<String>,
}

impl Default for UiEditorState {
    fn default() -> Self {
        Self {
            egui_id: egui::Id::new(uuid::Uuid::new_v4()),
            active_screen: None,
            selected: None,
            mode: Mode::Design,
            snap: true,
            zoom: 0.0,
            preview_data: UiData::default(),
            sample_json: String::new(),
            status: "Create a UI screen to begin".into(),
            error: None,
            search: String::new(),
            drag: None,
            history: history::History::default(),
            cached_code: None,
            delete_screen: false,
            selected_style: String::new(),
            import_texture: false,
            textures: UiTextures::default(),
            texture_sources: None,
            texture_error: None,
        }
    }
}

impl UiEditorState {
    fn activate(&mut self, screen: &UiScreen) {
        self.active_screen = Some(screen.id.clone());
        self.selected = screen.nodes.first().map(|n| n.id);
        self.preview_data = screen.sample_data.clone();
        self.sample_json = serde_json::to_string_pretty(&screen.sample_data).unwrap_or_default();
        self.mode = Mode::Design;
        self.drag = None;
        self.cached_code = None;
        self.delete_screen = false;
        self.selected_style.clear();
        self.import_texture = false;
        self.error = None;
        self.status = format!("Editing {}", screen.name);
    }

    fn sync(&mut self, screens: &[UiScreen]) {
        let current = screens
            .iter()
            .find(|s| Some(&s.id) == self.active_screen.as_ref());
        if let Some(screen) = current {
            if !screen.nodes.iter().any(|n| Some(n.id) == self.selected) {
                self.selected = None;
            }
        } else if let Some(screen) = screens.first() {
            self.activate(screen);
        } else {
            self.active_screen = None;
            self.selected = None;
            self.drag = None;
        }
    }

    fn refresh_data(&mut self, screen: &UiScreen) {
        self.preview_data = screen.sample_data.clone();
        self.sample_json = serde_json::to_string_pretty(&screen.sample_data).unwrap_or_default();
    }

    fn code(&mut self, screen: &UiScreen) -> &str {
        if self
            .cached_code
            .as_ref()
            .map(|(s, _)| s != screen)
            .unwrap_or(true)
        {
            self.cached_code = Some((
                screen.clone(),
                ui_codegen::source(screen).unwrap_or_else(|e| format!("// Cannot generate: {e}")),
            ));
        }
        &self.cached_code.as_ref().unwrap().1
    }
}

pub fn draw(ctx: &Context, root: &mut EditorRoot) {
    let Some(project) = root.project.as_mut() else {
        egui::CentralPanel::default().frame(theme::panel_frame()).show(ctx, |ui| {
            ui.add_space(40.0);
            ui.heading("Build your game's interface");
            ui.label("Open or create a project from File, then create a UI screen.");
            ui.add_space(12.0);
            ui.weak("Menus, HUDs, inventories, and conversations — authored here, driven by your game.");
        });
        return;
    };
    let state = &mut root.editor.ui_editor;
    state.sync(&project.ui_screens);
    let before = project.ui_screens.clone();
    let mut history_navigation = false;
    let mut generate = false;

    egui::TopBottomPanel::top("ui_document_toolbar")
        .frame(theme::panel_frame())
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Interface").strong().color(theme::ACCENT));
                ui.menu_button("+ Screen", |ui| {
                    for (label, starter) in
                        [("Blank screen", 0), ("Pause menu", 1), ("RPG dialogue", 2)]
                    {
                        if ui.button(label).clicked() {
                            let id = uuid::Uuid::new_v4().to_string();
                            let mut screen = match starter {
                                1 => UiScreen::pause_menu(id),
                                2 => UiScreen::dialogue(id),
                                _ => UiScreen::new("Screen", id),
                            };
                            screen.name = unique_name(
                                &screen.name,
                                project.ui_screens.iter().map(|s| s.name.as_str()),
                            );
                            state.activate(&screen);
                            project.ui_screens.push(screen);
                            ui.close_menu();
                        }
                    }
                });
                ui.separator();
                if ui
                    .add_enabled(state.history.can_undo(), egui::Button::new("Undo"))
                    .clicked()
                {
                    history_navigation = state.history.undo(&mut project.ui_screens);
                }
                if ui
                    .add_enabled(state.history.can_redo(), egui::Button::new("Redo"))
                    .clicked()
                {
                    history_navigation = state.history.redo(&mut project.ui_screens);
                }
                ui.separator();
                for (mode, label) in [
                    (Mode::Design, "Design"),
                    (Mode::Preview, "▶ Preview"),
                    (Mode::Code, "Code"),
                ] {
                    if ui.selectable_value(&mut state.mode, mode, label).changed() {
                        state.drag = None;
                        if let Some(screen) = project
                            .ui_screens
                            .iter()
                            .find(|s| Some(&s.id) == state.active_screen.as_ref())
                        {
                            state.preview_data = screen.sample_data.clone();
                        }
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    generate = ui
                        .add_enabled(
                            !project.ui_screens.is_empty(),
                            egui::Button::new(
                                RichText::new("Generate Rust").color(theme::BACKGROUND),
                            )
                            .fill(theme::ACCENT),
                        )
                        .clicked();
                });
            });
        });
    if !ctx.wants_keyboard_input() {
        if ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Z,
            )
        }) {
            history_navigation = state.history.redo(&mut project.ui_screens);
        } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
            history_navigation = state.history.undo(&mut project.ui_screens);
        }
    }
    state.sync(&project.ui_screens);
    if history_navigation {
        if let Some(screen) = restored_screen(&before, &project.ui_screens) {
            state.activate(screen);
        }
        if let Some(screen) = project
            .ui_screens
            .iter()
            .find(|s| Some(&s.id) == state.active_screen.as_ref())
        {
            state.refresh_data(screen);
        }
    }

    egui::SidePanel::left("ui_widgets")
        .default_width(210.0)
        .min_width(180.0)
        .frame(theme::panel_frame())
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    theme::panel_heading(ui, "UI screens", "Reusable interfaces for your game");
                    let mut activate = None;
                    for screen in &project.ui_screens {
                        if ui
                            .selectable_label(
                                Some(&screen.id) == state.active_screen.as_ref(),
                                &screen.name,
                            )
                            .clicked()
                        {
                            activate = Some(screen.id.clone());
                        }
                    }
                    if let Some(id) = activate {
                        state.history.finish(&project.ui_screens);
                        if let Some(screen) = project.ui_screens.iter().find(|s| s.id == id) {
                            state.activate(screen);
                        }
                    }
                    let Some(screen) = project
                        .ui_screens
                        .iter_mut()
                        .find(|s| Some(&s.id) == state.active_screen.as_ref())
                    else {
                        return;
                    };
                    ui.add_space(18.0);
                    theme::panel_heading(ui, "Widgets", "Add to the selected container");
                    crate::editor::widgets::search_field(ui, &mut state.search, "Find a widget…");
                    egui::Grid::new("ui_palette")
                        .num_columns(2)
                        .spacing([6.0, 6.0])
                        .show(ui, |ui| {
                            let mut count = 0;
                            for kind in WidgetKind::ALL {
                                if !crate::editor::widgets::matches_search(
                                    kind.label(),
                                    &state.search,
                                ) {
                                    continue;
                                }
                                if ui
                                    .add_sized([84.0, 30.0], egui::Button::new(kind.label()))
                                    .clicked()
                                {
                                    state.selected = Some(add_widget(screen, state.selected, kind));
                                    state.mode = Mode::Design;
                                    state.refresh_data(screen);
                                }
                                count += 1;
                                if count % 2 == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                    ui.add_space(18.0);
                    theme::panel_heading(ui, "Hierarchy", "Order controls drawing and stacking");
                    if ui
                        .selectable_label(
                            state.selected.is_none(),
                            format!("Screen: {}", screen.name),
                        )
                        .clicked()
                    {
                        state.selected = None;
                    }
                    hierarchy(ui, screen, None, 0, &mut state.selected);
                    if screen.nodes.is_empty() {
                        ui.weak("Add a widget to start this screen.");
                    }
                    ui.add_space(12.0);
                    if let Some(id) = state.selected {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Duplicate").clicked() {
                                state.selected = duplicate_node(screen, id);
                            }
                            if ui.button("Delete").clicked() {
                                screen.remove_node(id);
                                state.selected = None;
                            }
                            if ui
                                .small_button("Up")
                                .on_hover_text("Draw earlier")
                                .clicked()
                            {
                                reorder(screen, id, -1);
                            }
                            if ui
                                .small_button("Down")
                                .on_hover_text("Draw later")
                                .clicked()
                            {
                                reorder(screen, id, 1);
                            }
                        });
                    }
                });
        });
    state.sync(&project.ui_screens);
    let active_index = project
        .ui_screens
        .iter()
        .position(|s| Some(&s.id) == state.active_screen.as_ref());
    if let Some(index) = active_index {
        let texture_candidates: Vec<_> = project
            .manifest
            .iter()
            .filter(|entry| entry.kind == crate::editor::state::ResourceKind::Texture)
            .map(|entry| (entry.name.clone(), entry.relative_path.clone()))
            .collect();
        let screen = &mut project.ui_screens[index];
        if !ctx.wants_keyboard_input() && state.mode == Mode::Design {
            if ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)
                    || i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace)
            }) {
                if let Some(id) = state.selected.take() {
                    screen.remove_node(id);
                }
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::D)) {
                if let Some(id) = state.selected {
                    state.selected = duplicate_node(screen, id);
                }
            }
        }
        egui::SidePanel::right("ui_properties")
            .default_width(280.0)
            .min_width(240.0)
            .frame(theme::panel_frame())
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        inspector::draw(ui, screen, state, &texture_candidates)
                    });
            });
        egui::TopBottomPanel::bottom("ui_feedback")
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
                if let Some(error) = &state.texture_error {
                    ui.colored_label(theme::AMBER, error);
                }
                ui.horizontal_wrapped(|ui| {
                    ui.small(&state.status);
                    if let Err(error) = screen.validate() {
                        ui.colored_label(theme::AMBER, error);
                    }
                });
            });
        load_textures(ctx, screen, &project.root_path, state);
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BACKGROUND))
            .show(ctx, |ui| match state.mode {
                Mode::Design | Mode::Preview => canvas::draw(ui, screen, state),
                Mode::Code => code_panel(ui, screen, state),
            });
        if state.delete_screen {
            project.ui_screens.remove(index);
            state.delete_screen = false;
            state.sync(&project.ui_screens);
        }
    } else {
        egui::CentralPanel::default().frame(theme::panel_frame()).show(ctx, |ui| {
            ui.add_space(50.0);
            ui.heading("Make room for your game's UI");
            ui.label("Create a blank screen, a pause menu, or an RPG dialogue from + Screen.");
            ui.add_space(12.0);
            ui.weak("Bind widgets to game data. Repeat a template for choices, inventory items, or scores.");
        });
    }
    if state.import_texture {
        state.import_texture = false;
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Artwork", &["png", "jpg", "jpeg", "gif", "bmp", "tga"])
            .pick_file()
        {
            match import_artwork(project, state.active_screen.as_deref(), &path) {
                Ok(key) => {
                    state.error = None;
                    state.status = format!("Imported {key}. Choose it in Texture / sprite.");
                }
                Err(error) => state.error = Some(error),
            }
        }
    }
    if before != project.ui_screens {
        root.editor.dirty = true;
        root.editor.ui_revision = root.editor.ui_revision.wrapping_add(1);
        if !history_navigation {
            state.history.changed(before, &project.ui_screens);
        }
    }
    if !ctx.input(|i| i.pointer.any_down()) && !ctx.wants_keyboard_input() {
        state.history.finish(&project.ui_screens);
    }
    if generate {
        match ui_codegen::generate(
            std::path::Path::new(&project.root_path),
            &project.ui_screens,
        ) {
            Ok(paths) => {
                state.status = format!(
                    "Generated {} files in src/generated_ui. Open Code for registration.",
                    paths.len()
                );
                state.error = None;
                state.mode = Mode::Code;
            }
            Err(e) => state.error = Some(e),
        }
    }
}

fn load_textures(ctx: &Context, screen: &UiScreen, root: &str, state: &mut UiEditorState) {
    let sources = (screen.id.clone(), screen.textures.clone());
    if state.texture_sources.as_ref() == Some(&sources) {
        return;
    }
    state.texture_sources = Some(sources);
    state.textures.clear();
    state.texture_error = None;
    let mut total = 0usize;
    for (key, relative) in &screen.textures {
        let result = crate::project::ui_assets::load(std::path::Path::new(root), relative)
            .and_then(|asset| {
                total = total.saturating_add(asset.rgba.len());
                if total > 128 * 1024 * 1024 {
                    return Err(
                        "Screen artwork exceeds 128 MiB decoded. Use smaller textures.".into(),
                    );
                }
                state
                    .textures
                    .insert_rgba(ctx, key, asset.size, &asset.rgba)
            });
        if let Err(error) = result {
            state.texture_error = Some(format!("Texture {key}: {error}"));
        }
    }
}

fn import_artwork(
    project: &mut crate::editor::state::ProjectState,
    screen_id: Option<&str>,
    source: &std::path::Path,
) -> Result<String, String> {
    let index = project
        .ui_screens
        .iter()
        .position(|s| Some(s.id.as_str()) == screen_id)
        .ok_or("Select a screen before importing artwork.")?;
    if project.ui_screens[index].textures.len() >= 64 {
        return Err("A screen supports up to 64 textures.".into());
    }
    // Artwork imports write only into real resource directories in this project.
    // The shared resource importer predates UI assets and follows directory links.
    let mut destination = std::path::PathBuf::from(&project.root_path);
    for part in ["src", "resources", "textures"] {
        destination.push(part);
        match std::fs::symlink_metadata(&destination) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                return Err(format!(
                    "Expected a real directory at {}.",
                    destination.display()
                ))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    let metadata = std::fs::metadata(source).map_err(|err| err.to_string())?;
    if metadata.len() > 64 * 1024 * 1024 {
        return Err("Artwork file exceeds 64 MiB.".into());
    }
    let bytes = std::fs::read(source).map_err(|err| err.to_string())?;
    crate::project::ui_assets::decode(&bytes)?;
    let id = crate::project::resource::import(
        source,
        crate::editor::state::ResourceKind::Texture,
        project,
    )
    .map_err(|err| format!("Could not import artwork: {err:?}"))?;
    let entry = crate::project::resource::find(project, id)
        .ok_or("Imported texture is missing from project resources.")?;
    let (name, path) = (entry.name.clone(), entry.relative_path.clone());
    let screen = &mut project.ui_screens[index];
    let key = unique_name(&name, screen.textures.keys().map(String::as_str));
    screen.textures.insert(key.clone(), path);
    Ok(key)
}

fn code_panel(ui: &mut egui::Ui, screen: &UiScreen, state: &mut UiEditorState) {
    egui::Frame::none().inner_margin(16.0).show(ui, |ui| {
        ui.heading(format!("{}.rs", ui_codegen::module_name(screen)));
        ui.label("Generated layout and runtime data are separate from your game's event handler.");
        let class = ui_codegen::class_name(screen);
        let snippet = format!("// At the top of main.rs:\nmod generated_ui;\n\n// After creating app_state:\ngenerated_ui::{class}::install(&mut app_state, |event, app| {{\n    // event.name identifies the action; event.item_id identifies a choice.\n    println!(\"{{:?}}\", event);\n    // Update data via app.get_state_data_value_mut::<generated_ui::{class}>(\n    //     generated_ui::{class}::STATE_KEY);\n}});");
        egui::CollapsingHeader::new("Connect this screen to your game").default_open(true).show(ui, |ui| {
            if ui.button("Copy registration").clicked() { ui.output_mut(|o| o.copied_text = snippet.clone()); }
            egui::ScrollArea::horizontal().id_source("ui_registration").show(ui, |ui| { ui.monospace(&snippet); });
        });
        ui.separator();
        let code = state.code(screen).to_owned();
        if ui.button("Copy generated class").clicked() { ui.output_mut(|o| o.copied_text = code.clone()); }
        egui::ScrollArea::both().id_source("ui_source").auto_shrink([false, false]).show(ui, |ui| { ui.monospace(code); });
    });
}

fn hierarchy(
    ui: &mut egui::Ui,
    screen: &UiScreen,
    parent: Option<u64>,
    depth: usize,
    selected: &mut Option<u64>,
) {
    if depth > 32 {
        return;
    }
    for node in screen.nodes.iter().filter(|n| n.parent == parent) {
        ui.push_id(node.id, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(depth as f32 * 12.0);
                let prefix = if node.kind.is_container() { "-" } else { "·" };
                if ui
                    .selectable_label(
                        *selected == Some(node.id),
                        format!("{prefix} {}", node.name),
                    )
                    .on_hover_text(node.kind.label())
                    .clicked()
                {
                    *selected = Some(node.id);
                }
            });
        });
        hierarchy(ui, screen, Some(node.id), depth + 1, selected);
    }
}

fn unique_name<'a>(base: &str, names: impl Iterator<Item = &'a str>) -> String {
    let names: Vec<_> = names.collect();
    if !names.contains(&base) {
        return base.to_string();
    }
    for index in 2.. {
        let name = format!("{base}_{index}");
        if !names.contains(&name.as_str()) {
            return name;
        }
    }
    unreachable!()
}

fn add_widget(screen: &mut UiScreen, selected: Option<u64>, kind: WidgetKind) -> u64 {
    let parent = screen
        .nodes
        .iter()
        .find(|n| Some(n.id) == selected)
        .and_then(|n| {
            if n.kind.is_container() {
                Some(n.id)
            } else {
                n.parent
            }
        });
    let id = screen.next_id();
    let name = unique_name(
        &kind.label().to_lowercase().replace(' ', "_"),
        screen.nodes.iter().map(|n| n.name.as_str()),
    );
    let mut node = UiNode::new(id, kind, name);
    let style = appearance::role(kind);
    if screen.styles.contains_key(style) {
        node.style_key = style.into();
    }
    node.parent = parent;
    node.position = if parent.is_some() {
        [16.0, 16.0]
    } else {
        [48.0, 48.0]
    };
    if kind == WidgetKind::Repeater {
        node.list_binding =
            unique_name("items", screen.sample_data.lists.keys().map(String::as_str));
        screen.sample_data.set_items(
            &node.list_binding,
            vec![
                UiItem::new("first").with_text("text", "First item"),
                UiItem::new("second").with_text("text", "Second item"),
            ],
        );
        let mut child = UiNode::new(
            id + 1,
            WidgetKind::Button,
            unique_name("item", screen.nodes.iter().map(|n| n.name.as_str())),
        );
        if screen.styles.contains_key("button") {
            child.style_key = "button".into();
        }
        child.parent = Some(id);
        child.position = [4.0, 4.0];
        child.size = [node.size[0] - 24.0, (node.row_height - 8.0).max(24.0)];
        child.text = "Item template".into();
        child.binding = "text".into();
        child.event = "select_item".into();
        screen.nodes.push(node);
        screen.nodes.push(child);
    } else {
        screen.nodes.push(node);
    }
    id
}

fn duplicate_node(screen: &mut UiScreen, id: u64) -> Option<u64> {
    if !screen.nodes.iter().any(|n| n.id == id) {
        return None;
    }
    let mut ids = vec![id];
    for _ in 0..screen.nodes.len() {
        let old = ids.len();
        for node in &screen.nodes {
            if node.parent.map(|p| ids.contains(&p)).unwrap_or(false) && !ids.contains(&node.id) {
                ids.push(node.id);
            }
        }
        if old == ids.len() {
            break;
        }
    }
    let start = screen.next_id();
    let copies: Vec<_> = screen
        .nodes
        .iter()
        .filter(|n| ids.contains(&n.id))
        .cloned()
        .collect();
    let map: std::collections::HashMap<_, _> = copies
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id, start + i as u64))
        .collect();
    for mut node in copies {
        let original_id = node.id;
        node.id = map[&original_id];
        node.name = unique_name(
            &format!("{}_copy", node.name),
            screen.nodes.iter().map(|n| n.name.as_str()),
        );
        if let Some(parent) = node.parent {
            node.parent = Some(*map.get(&parent).unwrap_or(&parent));
        }
        if original_id == id {
            node.position[0] += 16.0;
            node.position[1] += 16.0;
        }
        screen.nodes.push(node);
    }
    Some(map[&id])
}

fn reorder(screen: &mut UiScreen, id: u64, direction: i32) {
    let Some(index) = screen.nodes.iter().position(|n| n.id == id) else {
        return;
    };
    let siblings: Vec<_> = screen
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.parent == screen.nodes[index].parent)
        .map(|(i, _)| i)
        .collect();
    let position = siblings.iter().position(|i| *i == index).unwrap() as i32 + direction;
    if position >= 0 && (position as usize) < siblings.len() {
        screen.nodes.swap(index, siblings[position as usize]);
    }
}

fn restored_screen<'a>(before: &[UiScreen], after: &'a [UiScreen]) -> Option<&'a UiScreen> {
    after
        .iter()
        .find(|screen| before.iter().find(|old| old.id == screen.id) != Some(*screen))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn artwork_import_validates_before_mutating_and_preserves_both_files() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = crate::editor::state::ProjectState::new(
            "Art".into(),
            tmp.path().to_string_lossy().into_owned(),
        );
        project.ui_screens.push(UiScreen::new("Art", "art"));
        let source = tmp.path().join("portrait.png");
        std::fs::write(&source, b"invalid image").unwrap();
        assert!(import_artwork(&mut project, Some("art"), &source).is_err());
        assert!(project.manifest.is_empty());
        assert!(project.ui_screens[0].textures.is_empty());

        image::RgbaImage::from_pixel(2, 2, image::Rgba([200, 80, 40, 255]))
            .save(&source)
            .unwrap();
        let first = import_artwork(&mut project, Some("art"), &source).unwrap();
        let second = import_artwork(&mut project, Some("art"), &source).unwrap();
        assert_ne!(first, second);
        assert_eq!(project.ui_screens[0].textures.len(), 2);
        assert!(source.exists());
        for path in project.ui_screens[0].textures.values() {
            assert_eq!(
                crate::project::ui_assets::load(tmp.path(), path)
                    .unwrap()
                    .size,
                [2, 2]
            );
        }
    }

    #[test]
    fn newly_added_widgets_use_the_screens_matching_named_style() {
        let mut screen = UiScreen::new("Styled", "styled");
        screen
            .styles
            .insert("button".into(), crate::ui_runtime::UiStyle::default());
        let button = add_widget(&mut screen, None, WidgetKind::Button);
        assert_eq!(
            screen
                .nodes
                .iter()
                .find(|n| n.id == button)
                .unwrap()
                .style_key,
            "button"
        );
        let label = add_widget(&mut screen, None, WidgetKind::Label);
        assert!(screen
            .nodes
            .iter()
            .find(|n| n.id == label)
            .unwrap()
            .style_key
            .is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn artwork_import_refuses_a_linked_destination_without_writing() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src/resources")).unwrap();
        std::os::unix::fs::symlink(outside.path(), tmp.path().join("src/resources/textures"))
            .unwrap();
        let mut project = crate::editor::state::ProjectState::new(
            "Art".into(),
            tmp.path().to_string_lossy().into_owned(),
        );
        project.ui_screens.push(UiScreen::new("Art", "art"));
        let source = tmp.path().join("portrait.png");
        image::RgbaImage::from_pixel(1, 1, image::Rgba([255; 4]))
            .save(&source)
            .unwrap();
        assert!(import_artwork(&mut project, Some("art"), &source).is_err());
        assert!(project.manifest.is_empty());
        assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    }

    #[test]
    fn history_focus_follows_the_screen_that_was_restored() {
        let a = UiScreen::new("A", "a");
        let mut b = UiScreen::new("B", "b");
        let original = vec![a.clone(), b.clone()];
        b.name = "Edited B".into();
        let edited = vec![a, b];
        assert_eq!(
            restored_screen(&edited, &original).map(|s| s.id.as_str()),
            Some("b")
        );
        assert_eq!(
            restored_screen(&original[..1], &original).map(|s| s.id.as_str()),
            Some("b")
        );
    }
    #[test]
    fn duplicate_container_remaps_children_and_does_not_alias_original() {
        let mut screen = UiScreen::new("test", "screen");
        let parent = add_widget(&mut screen, None, WidgetKind::Panel);
        let child = add_widget(&mut screen, Some(parent), WidgetKind::Button);
        let copy = duplicate_node(&mut screen, parent).unwrap();
        let copied_child = screen
            .nodes
            .iter()
            .find(|n| n.parent == Some(copy))
            .unwrap();
        assert_ne!(copied_child.id, child);
        assert_ne!(
            copied_child.name,
            screen.nodes.iter().find(|n| n.id == child).unwrap().name
        );
        screen.remove_node(copy);
        assert_eq!(screen.nodes.len(), 2);
        screen.validate().unwrap();
    }
    #[test]
    fn activating_screen_resets_preview_without_mutating_authored_data() {
        let screen = UiScreen::dialogue("one");
        let mut state = UiEditorState::default();
        state.activate(&screen);
        state.preview_data.set_text("speaker", "Runtime change");
        assert_ne!(state.preview_data, screen.sample_data);
        state.activate(&screen);
        assert_eq!(state.preview_data, screen.sample_data);
        state.sync(&[]);
        assert!(state.active_screen.is_none());
        assert!(state.selected.is_none());
    }
}
