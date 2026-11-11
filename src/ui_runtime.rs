// This file also ships verbatim with generated UI modules.
use enigma_3d::ui as egui;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MAX_NODES: usize = 1024;
const MAX_DEPTH: usize = 32;
const MAX_ROWS: usize = 4096;
const MAX_DATA_ENTRIES: usize = 32768;
const MAX_DRAW_VISITS: usize = 8192;
const MAX_COORD: f32 = 1_000_000.0;
const MAX_KEY: usize = 256;
const MAX_TEXT: usize = 65536;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WidgetKind {
    Panel,
    Label,
    Button,
    Toggle,
    Slider,
    Progress,
    TextInput,
    Repeater,
    Image,
}

impl WidgetKind {
    pub const ALL: [Self; 9] = [
        Self::Panel,
        Self::Label,
        Self::Button,
        Self::Toggle,
        Self::Slider,
        Self::Progress,
        Self::TextInput,
        Self::Repeater,
        Self::Image,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Panel => "Panel",
            Self::Label => "Label",
            Self::Button => "Button",
            Self::Toggle => "Toggle",
            Self::Slider => "Slider",
            Self::Progress => "Progress",
            Self::TextInput => "Text Input",
            Self::Repeater => "Repeater",
            Self::Image => "Image",
        }
    }

    pub fn is_container(self) -> bool {
        matches!(self, Self::Panel | Self::Repeater)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiScreen {
    pub id: String,
    pub name: String,
    pub size: [f32; 2],
    #[serde(default, deserialize_with = "deserialize_nodes")]
    pub nodes: Vec<UiNode>,
    #[serde(default)]
    pub sample_data: UiData,
    #[serde(default)]
    pub textures: BTreeMap<String, String>,
    #[serde(default)]
    pub styles: BTreeMap<String, UiStyle>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiNode {
    pub id: u64,
    pub name: String,
    pub kind: WidgetKind,
    #[serde(default)]
    pub parent: Option<u64>,
    #[serde(default)]
    pub position: [f32; 2],
    #[serde(default = "default_size")]
    pub size: [f32; 2],
    #[serde(default)]
    pub anchor: [f32; 2],
    #[serde(default)]
    pub text: String,
    #[serde(default = "default_color")]
    pub color: [u8; 4],
    #[serde(default = "default_text_color")]
    pub text_color: [u8; 4],
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default)]
    pub rounding: f32,
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub binding: String,
    #[serde(default)]
    pub visible_binding: String,
    #[serde(default)]
    pub enabled_binding: String,
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub value: f32,
    #[serde(default = "default_range")]
    pub range: [f32; 2],
    #[serde(default)]
    pub checked: bool,
    #[serde(default)]
    pub list_binding: String,
    #[serde(default = "default_row_height")]
    pub row_height: f32,
    #[serde(default)]
    pub gap: f32,
    #[serde(default)]
    pub style_key: String,
    #[serde(default)]
    pub skin: UiSkin,
}

fn deserialize_nodes<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<UiNode>, D::Error> {
    if !deserializer.is_human_readable() {
        return Vec::deserialize(deserializer);
    }
    #[derive(Deserialize)]
    struct SavedNode {
        #[serde(flatten)]
        node: UiNode,
        skin: Option<UiSkin>,
    }
    Ok(Vec::<SavedNode>::deserialize(deserializer)?
        .into_iter()
        .map(|SavedNode { mut node, skin }| {
            if let Some(skin) = skin {
                node.skin = skin;
            } else if matches!(node.kind, WidgetKind::Label | WidgetKind::Repeater)
                && node.color == default_color()
            {
                // Pre-skin files stored the widget fill color on transparent
                // labels and repeaters. Persist their actual appearance.
                node.color = [0; 4];
            }
            node
        })
        .collect())
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ImageFit {
    #[default]
    Stretch,
    Contain,
    Cover,
    NineSlice,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum UiFont {
    #[default]
    Proportional,
    Monospace,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum UiTextAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSkin {
    pub texture: String,
    pub hover_texture: String,
    pub pressed_texture: String,
    pub image_binding: String,
    /// Normalized sprite rectangle, measured from the texture's upper left.
    pub uv: [f32; 4],
    pub fit: ImageFit,
    /// Left, top, right, bottom margins in source pixels.
    pub nine_slice: [f32; 4],
    pub tint: [u8; 4],
    pub pixelated: bool,
    pub border_color: [u8; 4],
    pub border_width: f32,
    pub padding: f32,
    pub font: UiFont,
    pub align: UiTextAlign,
    pub hover_color: Option<[u8; 4]>,
    pub pressed_color: Option<[u8; 4]>,
}

impl Default for UiSkin {
    fn default() -> Self {
        Self {
            texture: String::new(),
            hover_texture: String::new(),
            pressed_texture: String::new(),
            image_binding: String::new(),
            uv: [0.0, 0.0, 1.0, 1.0],
            fit: ImageFit::Stretch,
            nine_slice: [0.0; 4],
            tint: [255; 4],
            pixelated: false,
            border_color: [0; 4],
            border_width: 0.0,
            padding: 0.0,
            font: UiFont::Proportional,
            align: UiTextAlign::Left,
            hover_color: None,
            pressed_color: None,
        }
    }
}

impl UiSkin {
    fn validate(&self, textures: &BTreeMap<String, String>) -> Result<(), &'static str> {
        if !self.valid_geometry() {
            return Err("invalid sprite bounds, slice margins, padding, or border width");
        }
        if !self.image_binding.is_empty() && !valid_key(&self.image_binding) {
            return Err("invalid image binding");
        }
        for key in [&self.texture, &self.hover_texture, &self.pressed_texture] {
            if !key.is_empty() && !textures.contains_key(key) {
                return Err("references a missing texture asset");
            }
        }
        Ok(())
    }

    fn valid_geometry(&self) -> bool {
        let [u0, v0, u1, v1] = self.uv;
        (0.0..u1).contains(&u0)
            && (0.0..v1).contains(&v0)
            && u1 <= 1.0
            && v1 <= 1.0
            && self
                .nine_slice
                .iter()
                .chain([&self.border_width, &self.padding])
                .all(|v| (0.0..=MAX_COORD).contains(v))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiStyle {
    pub color: [u8; 4],
    pub text_color: [u8; 4],
    pub font_size: f32,
    pub rounding: f32,
    pub skin: UiSkin,
}

impl Default for UiStyle {
    fn default() -> Self {
        Self {
            color: default_color(),
            text_color: default_text_color(),
            font_size: default_font_size(),
            rounding: 0.0,
            skin: UiSkin::default(),
        }
    }
}

impl UiStyle {
    pub fn from_node(node: &UiNode) -> Self {
        Self {
            color: node.color,
            text_color: node.text_color,
            font_size: node.font_size,
            rounding: node.rounding,
            skin: node.skin.clone(),
        }
    }
}

fn default_size() -> [f32; 2] {
    [160.0, 44.0]
}
fn default_color() -> [u8; 4] {
    [44, 56, 75, 255]
}
fn default_text_color() -> [u8; 4] {
    [229, 237, 248, 255]
}
fn default_font_size() -> f32 {
    19.0
}
fn default_range() -> [f32; 2] {
    [0.0, 1.0]
}
fn default_row_height() -> f32 {
    46.0
}
fn yes() -> bool {
    true
}

impl UiNode {
    pub fn new(id: u64, kind: WidgetKind, name: impl Into<String>) -> Self {
        let name = name.into();
        let size = match kind {
            WidgetKind::Panel => [300.0, 220.0],
            WidgetKind::Repeater => [220.0, 160.0],
            WidgetKind::Image => [160.0, 120.0],
            _ => default_size(),
        };
        Self {
            id,
            text: match kind {
                WidgetKind::Label | WidgetKind::Button | WidgetKind::Toggle => name.clone(),
                _ => String::new(),
            },
            name,
            kind,
            parent: None,
            position: [0.0; 2],
            size,
            anchor: [0.0; 2],
            color: if matches!(
                kind,
                WidgetKind::Label | WidgetKind::Repeater | WidgetKind::Image
            ) {
                [0; 4]
            } else {
                default_color()
            },
            text_color: default_text_color(),
            font_size: default_font_size(),
            rounding: 6.0,
            visible: true,
            enabled: true,
            binding: String::new(),
            visible_binding: String::new(),
            enabled_binding: String::new(),
            event: String::new(),
            value: 0.5,
            range: default_range(),
            checked: false,
            list_binding: String::new(),
            row_height: default_row_height(),
            gap: 6.0,
            style_key: String::new(),
            skin: UiSkin {
                padding: if matches!(
                    kind,
                    WidgetKind::Button
                        | WidgetKind::Toggle
                        | WidgetKind::Slider
                        | WidgetKind::Progress
                        | WidgetKind::TextInput
                ) {
                    8.0
                } else {
                    0.0
                },
                align: if kind == WidgetKind::Button {
                    UiTextAlign::Center
                } else {
                    UiTextAlign::Left
                },
                ..UiSkin::default()
            },
        }
    }

    fn has_valid_geometry(&self) -> bool {
        self.position.iter().all(|v| v.abs() <= MAX_COORD)
            && self.size.iter().copied().all(positive_size)
            && self.anchor.iter().all(|v| (0.0..=1.0).contains(v))
            && positive_size(self.font_size)
            && (0.0..=MAX_COORD).contains(&self.rounding)
            && self.value.is_finite()
            && self.range.iter().all(|v| v.is_finite())
            && self.range[0] < self.range[1]
            && (self.range[1] - self.range[0]).is_finite()
            && positive_size(self.row_height)
            && (0.0..=MAX_COORD).contains(&self.gap)
    }
}

impl UiScreen {
    pub fn new(name: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            size: [1280.0, 720.0],
            nodes: Vec::new(),
            sample_data: UiData::default(),
            textures: BTreeMap::new(),
            styles: BTreeMap::new(),
        }
    }

    pub fn dialogue(id: impl Into<String>) -> Self {
        let mut screen = Self::new("Dialogue", id);
        let mut panel = UiNode::new(1, WidgetKind::Panel, "Dialogue panel");
        panel.anchor = [0.5, 1.0];
        panel.position = [0.0, -36.0];
        panel.size = [920.0, 304.0];
        panel.color = [22, 32, 46, 246];
        panel.rounding = 15.0;
        let mut speaker = child(
            2,
            WidgetKind::Label,
            "Speaker",
            1,
            [28.0, 20.0],
            [864.0, 32.0],
        );
        speaker.text = "Warden Iona".into();
        speaker.binding = "speaker".into();
        speaker.font_size = 26.0;
        speaker.text_color = [132, 181, 250, 255];
        let mut line = child(
            3,
            WidgetKind::Label,
            "Dialogue",
            1,
            [28.0, 56.0],
            [864.0, 70.0],
        );
        line.text = "The crossing is closed after sundown. What brings you out here?".into();
        line.binding = "dialogue".into();
        let mut choices = child(
            4,
            WidgetKind::Repeater,
            "Choices",
            1,
            [28.0, 133.0],
            [864.0, 150.0],
        );
        choices.list_binding = "choices".into();
        choices.row_height = 44.0;
        choices.gap = 7.0;
        let mut choice = child(
            5,
            WidgetKind::Button,
            "Choice",
            4,
            [0.0, 0.0],
            [840.0, 44.0],
        );
        choice.text = "Ask about the crossing".into();
        choice.binding = "text".into();
        choice.enabled_binding = "enabled".into();
        choice.event = "choose".into();
        screen.sample_data.set_text("speaker", &speaker.text);
        screen.sample_data.set_text("dialogue", &line.text);
        screen.sample_data.set_items(
            "choices",
            vec![
                UiItem::new("crossing")
                    .with_text("text", &choice.text)
                    .with_bool("enabled", true),
                UiItem::new("help")
                    .with_text("text", "Offer to help")
                    .with_bool("enabled", true),
                UiItem::new("leave")
                    .with_text("text", "Leave")
                    .with_bool("enabled", true),
            ],
        );
        screen.nodes = vec![panel, speaker, line, choices, choice];
        screen
    }

    pub fn pause_menu(id: impl Into<String>) -> Self {
        let mut screen = Self::new("Pause menu", id);
        let mut panel = UiNode::new(1, WidgetKind::Panel, "Menu panel");
        panel.anchor = [0.5, 0.5];
        panel.size = [480.0, 488.0];
        panel.color = [24, 32, 45, 250];
        panel.rounding = 15.0;
        let mut title = child(
            2,
            WidgetKind::Label,
            "Heading",
            1,
            [60.0, 50.0],
            [360.0, 50.0],
        );
        title.text = "Take a breather.".into();
        title.font_size = 34.0;
        let mut subtitle = child(
            3,
            WidgetKind::Label,
            "Subtitle",
            1,
            [60.0, 118.0],
            [360.0, 32.0],
        );
        subtitle.text = "Your expedition will be here when you return.".into();
        subtitle.font_size = 16.0;
        subtitle.text_color = [163, 177, 195, 255];

        let button = |id, name, text: &str, event: &str, y| {
            let mut node = child(id, WidgetKind::Button, name, 1, [60.0, y], [360.0, 54.0]);
            node.text = text.into();
            node.event = event.into();
            node
        };
        let mut resume = button(
            4,
            "Resume button",
            "Resume expedition",
            "ResumeClicked",
            192.0,
        );
        resume.color = [132, 181, 250, 255];
        resume.text_color = [21, 32, 48, 255];
        let settings = button(5, "Settings button", "Settings", "SettingsClicked", 262.0);
        let exit = button(6, "Exit button", "Return to title", "ExitClicked", 332.0);
        let mut chapter = UiNode::new(7, WidgetKind::Label, "Chapter label");
        chapter.position = [48.0, 48.0];
        chapter.size = [440.0, 32.0];
        chapter.text = "NORTHSTAR  /  THE CROSSING".into();
        chapter.font_size = 16.0;
        chapter.text_color = [169, 193, 209, 255];
        screen.nodes = vec![panel, title, subtitle, resume, settings, exit, chapter];
        screen
    }

    pub fn next_id(&self) -> u64 {
        let max = self.nodes.iter().map(|node| node.id).max().unwrap_or(0);
        max.checked_add(1).unwrap_or_else(|| {
            // A loaded layout may already contain u64::MAX.
            (1..)
                .find(|id| !self.nodes.iter().any(|node| node.id == *id))
                .unwrap()
        })
    }

    pub fn remove_node(&mut self, id: u64) {
        let mut children: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
        for node in &self.nodes {
            if let Some(parent) = node.parent {
                children.entry(parent).or_default().push(node.id);
            }
        }
        let mut removed = BTreeSet::new();
        let mut pending = vec![id];
        while let Some(id) = pending.pop() {
            if removed.insert(id) {
                pending.extend(children.remove(&id).into_iter().flatten());
            }
        }
        self.nodes.retain(|node| !removed.contains(&node.id));
    }

    pub fn validate(&self) -> Result<(), String> {
        if !valid_key(&self.name) || !valid_key(&self.id) {
            return Err(
                "A UI screen needs a nonempty name and ID without control characters.".into(),
            );
        }
        if !self.size.into_iter().all(positive_size) {
            return Err("Screen dimensions must be positive and finite.".into());
        }
        Graph::new(&self.nodes)?;
        if self.textures.len() > 64 || self.styles.len() > MAX_NODES {
            return Err("Too many texture assets or styles in a screen.".into());
        }
        for (key, path) in &self.textures {
            if !valid_key(key) || path.len() > 1024 || !is_resource_path(path) {
                return Err(format!(
                    "Texture {key:?} needs a relative path in src/resources."
                ));
            }
        }
        for (key, style) in &self.styles {
            if !valid_key(key)
                || !positive_size(style.font_size)
                || !(0.0..=MAX_COORD).contains(&style.rounding)
            {
                return Err(format!("Invalid UI style {key:?}."));
            }
            style
                .skin
                .validate(&self.textures)
                .map_err(|err| format!("Style {key:?}: {err}."))?;
        }
        let mut names = BTreeSet::new();
        for node in &self.nodes {
            if !valid_key(&node.name) || !names.insert(node.name.trim()) {
                return Err(format!(
                    "Widget name is invalid or repeated: {:?}",
                    node.name
                ));
            }
            for key in [
                &node.binding,
                &node.visible_binding,
                &node.enabled_binding,
                &node.event,
                &node.list_binding,
            ] {
                if !key.is_empty() && !valid_key(key) {
                    return Err(format!(
                        "Widget {:?} has an invalid binding or event name.",
                        node.name
                    ));
                }
            }
            if node.text.len() > MAX_TEXT || node.text.contains('\0') {
                return Err(format!(
                    "Widget {:?} has invalid or oversized text.",
                    node.name
                ));
            }
            if !node.style_key.is_empty() && !self.styles.contains_key(&node.style_key) {
                return Err(format!(
                    "Widget {:?} references missing style {:?}.",
                    node.name, node.style_key
                ));
            }
            node.skin
                .validate(&self.textures)
                .map_err(|err| format!("Widget {:?}: {err}.", node.name))?;
        }
        let mut remaining = MAX_DATA_ENTRIES;
        validate_data(&self.sample_data, 0, &mut remaining)
    }
}

fn child(
    id: u64,
    kind: WidgetKind,
    name: &str,
    parent: u64,
    position: [f32; 2],
    size: [f32; 2],
) -> UiNode {
    let mut node = UiNode::new(id, kind, name);
    node.parent = Some(parent);
    node.position = position;
    node.size = size;
    node
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiData {
    pub text: BTreeMap<String, String>,
    pub numbers: BTreeMap<String, f32>,
    pub flags: BTreeMap<String, bool>,
    pub lists: BTreeMap<String, Vec<UiItem>>,
}

impl UiData {
    pub fn set_text(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.text.insert(key.into(), value.into());
    }

    pub fn set_number(&mut self, key: impl Into<String>, value: f32) {
        self.numbers.insert(key.into(), value);
    }

    pub fn set_bool(&mut self, key: impl Into<String>, value: bool) {
        self.flags.insert(key.into(), value);
    }

    pub fn set_items(&mut self, key: impl Into<String>, items: Vec<UiItem>) {
        self.lists.insert(key.into(), items);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiItem {
    pub id: String,
    #[serde(default)]
    pub data: UiData,
}

impl UiItem {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            data: UiData::default(),
        }
    }

    pub fn with_text(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.set_text(key, value);
        self
    }

    pub fn with_bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.data.set_bool(key, value);
        self
    }

    pub fn with_number(mut self, key: impl Into<String>, value: f32) -> Self {
        self.data.set_number(key, value);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum UiEventValue {
    Click,
    Bool(bool),
    Number(f32),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiEvent {
    pub name: String,
    pub widget: String,
    pub item_id: Option<String>,
    #[serde(default)]
    pub item_path: Vec<String>,
    pub value: UiEventValue,
}

fn positive_size(size: f32) -> bool {
    (0.001..=MAX_COORD).contains(&size)
}

fn valid_key(key: &str) -> bool {
    !key.trim().is_empty() && key.len() <= MAX_KEY && !key.chars().any(char::is_control)
}

pub(crate) fn is_resource_path(path: &str) -> bool {
    use std::path::{Component, Path};
    !path.trim().is_empty()
        && !path
            .chars()
            .any(|c| c == '\\' || c == ':' || c.is_control())
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn validate_data(data: &UiData, depth: usize, remaining: &mut usize) -> Result<(), String> {
    let entries = data
        .text
        .len()
        .saturating_add(data.numbers.len())
        .saturating_add(data.flags.len())
        .saturating_add(data.lists.len());
    if depth > MAX_DEPTH || entries > *remaining {
        return Err("Sample data has too many entries or nested lists.".into());
    }
    *remaining -= entries;
    let keys = data
        .text
        .keys()
        .chain(data.numbers.keys())
        .chain(data.flags.keys())
        .chain(data.lists.keys());
    if keys.into_iter().any(|key| !valid_key(key))
        || data.numbers.values().any(|n| !n.is_finite())
        || data
            .text
            .values()
            .any(|s| s.len() > MAX_TEXT || s.contains('\0'))
    {
        return Err("Sample data contains an invalid key, number, or text value.".into());
    }
    for (key, rows) in &data.lists {
        if rows.len() > MAX_ROWS || rows.len() > *remaining {
            return Err(format!("Sample list {key:?} contains too many rows."));
        }
        *remaining -= rows.len();
        let mut ids = BTreeSet::new();
        for row in rows {
            if !valid_key(&row.id) || !ids.insert(&row.id) {
                return Err(format!(
                    "Sample list {key:?} contains an invalid or repeated row ID."
                ));
            }
            validate_data(&row.data, depth + 1, remaining)?;
        }
    }
    Ok(())
}

struct Graph<'a> {
    nodes: &'a [UiNode],
    children: BTreeMap<Option<u64>, Vec<usize>>,
}

impl<'a> Graph<'a> {
    fn new(nodes: &'a [UiNode]) -> Result<Self, String> {
        if nodes.len() > MAX_NODES {
            return Err(format!("A UI screen may have up to {MAX_NODES} widgets."));
        }
        let mut by_id = BTreeMap::new();
        let mut children: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (index, node) in nodes.iter().enumerate() {
            if by_id.insert(node.id, index).is_some() {
                return Err(format!("Widget ID {} is repeated.", node.id));
            }
            if !node.has_valid_geometry() {
                return Err(format!(
                    "Widget {:?} has invalid geometry or value range.",
                    node.name
                ));
            }
            children.entry(node.parent).or_default().push(index);
        }
        for node in nodes {
            let mut parent = node.parent;
            // Bound this walk even on unvalidated graphs. Any longer chain is
            // either a cycle or too deep to render.
            for _ in 0..MAX_DEPTH {
                let Some(id) = parent else {
                    break;
                };
                let parent_node = by_id
                    .get(&id)
                    .map(|&index| &nodes[index])
                    .ok_or_else(|| format!("Widget {:?} has a missing parent.", node.name))?;
                if !parent_node.kind.is_container() {
                    return Err(format!(
                        "Widget {:?} has a parent that cannot contain widgets.",
                        node.name
                    ));
                }
                parent = parent_node.parent;
            }
            if parent.is_some() {
                return Err(format!(
                    "Widget {:?} is in a cycle or is nested too deeply.",
                    node.name
                ));
            }
        }
        Ok(Self { nodes, children })
    }

    fn children(&self, parent: Option<u64>) -> &[usize] {
        self.children
            .get(&parent)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

/// Positions are in reference points relative to the matching parent and widget pivots.
pub fn node_rect(node: &UiNode, parent: egui::Rect, scale: f32) -> egui::Rect {
    let position = egui::vec2(node.position[0], node.position[1]) * scale;
    let size = egui::vec2(node.size[0], node.size[1]) * scale;
    let anchor = egui::vec2(node.anchor[0], node.anchor[1]);
    egui::Rect::from_min_size(
        parent.min + anchor * parent.size() + position - anchor * size,
        size,
    )
}

fn fit(screen: &UiScreen, rect: egui::Rect) -> Option<(egui::Rect, f32)> {
    if !screen.size.into_iter().all(positive_size) || !rect.is_finite() || !rect.is_positive() {
        return None;
    }
    let scale = (rect.width() / screen.size[0]).min(rect.height() / screen.size[1]);
    let size = egui::vec2(screen.size[0], screen.size[1]) * scale;
    (scale.is_finite() && scale > 0.0)
        .then(|| (egui::Rect::from_center_size(rect.center(), size), scale))
}

/// Editable rectangles in painter order. A repeater contributes one template row.
pub fn layout_nodes(screen: &UiScreen, rect: egui::Rect) -> Vec<(u64, egui::Rect)> {
    let Some((rect, scale)) = fit(screen, rect) else {
        return Vec::new();
    };
    let Ok(graph) = Graph::new(&screen.nodes) else {
        return Vec::new();
    };
    fn collect(
        graph: &Graph<'_>,
        parent: Option<u64>,
        parent_rect: egui::Rect,
        scale: f32,
        out: &mut Vec<(u64, egui::Rect)>,
    ) {
        for &index in graph.children(parent) {
            let node = &graph.nodes[index];
            let rect = node_rect(node, parent_rect, scale);
            if !rect.is_finite() {
                continue;
            }
            out.push((node.id, rect));
            let child_rect = if node.kind == WidgetKind::Repeater {
                row_rect(rect, 0.0, node.row_height * scale)
            } else {
                rect
            };
            collect(graph, Some(node.id), child_rect, scale, out);
        }
    }
    let mut nodes = Vec::with_capacity(screen.nodes.len());
    collect(&graph, None, rect, scale, &mut nodes);
    nodes
}

fn row_rect(rect: egui::Rect, offset: f32, height: f32) -> egui::Rect {
    egui::Rect::from_min_size(
        rect.min + egui::vec2(0.0, offset),
        egui::vec2(rect.width(), height),
    )
}

#[derive(Clone, Default)]
pub struct UiTextures {
    entries: BTreeMap<String, UiTexture>,
}

#[derive(Clone)]
struct UiTexture {
    size: [usize; 2],
    smooth: egui::TextureHandle,
    pixelated: egui::TextureHandle,
}

impl UiTextures {
    pub fn insert_rgba(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        size: [usize; 2],
        pixels: &[u8],
    ) -> Result<(), String> {
        let max_side = ctx.input(|input| input.max_texture_side).min(4096);
        if !valid_key(key)
            || size.iter().any(|&side| side == 0 || side > max_side)
            || size[0].checked_mul(size[1]).and_then(|n| n.checked_mul(4)) != Some(pixels.len())
        {
            return Err(format!("Invalid RGBA texture {key:?}: expected nonzero dimensions up to {max_side} and four bytes per pixel."));
        }
        let image = egui::ColorImage::from_rgba_unmultiplied(size, pixels);
        let smooth = ctx.load_texture(key, image.clone(), egui::TextureOptions::LINEAR);
        let pixelated = ctx.load_texture(key, image, egui::TextureOptions::NEAREST);
        self.entries.insert(
            key.into(),
            UiTexture {
                size,
                smooth,
                pixelated,
            },
        );
        Ok(())
    }

    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Paints a design template or runs its interactive UI, returning only configured events.
pub fn draw_screen(
    ui: &mut egui::Ui,
    screen: &UiScreen,
    data: &mut UiData,
    rect: egui::Rect,
    interactive: bool,
) -> Vec<UiEvent> {
    draw_screen_with_textures(ui, screen, data, rect, interactive, &UiTextures::default())
}

pub fn draw_screen_with_textures(
    ui: &mut egui::Ui,
    screen: &UiScreen,
    data: &mut UiData,
    rect: egui::Rect,
    interactive: bool,
    textures: &UiTextures,
) -> Vec<UiEvent> {
    let Some((rect, scale)) = fit(screen, rect) else {
        return Vec::new();
    };
    let Ok(graph) = Graph::new(&screen.nodes) else {
        return Vec::new();
    };
    let mut renderer = Renderer {
        graph,
        data,
        styles: &screen.styles,
        textures,
        screen_id: &screen.id,
        id_prefix: ui.make_persistent_id(&screen.id),
        scale,
        interactive,
        visits_left: MAX_DRAW_VISITS,
        item_path: Vec::new(),
        events: Vec::new(),
    };
    renderer.draw_children(
        ui,
        None,
        rect,
        rect.intersect(ui.clip_rect()),
        &[],
        ui.is_enabled(),
    );
    renderer.events
}

// A scope records the physical source of each list. The separate item path
// describes the logical widget instance, including nested lists read from root.
type Scope<'a> = [(&'a str, usize)];

fn scoped_data<'d>(mut data: &'d UiData, scope: &Scope<'_>) -> Option<&'d UiData> {
    for &(key, index) in scope {
        data = &data.lists.get(key)?.get(index)?.data;
    }
    Some(data)
}

fn scoped_data_mut<'d>(mut data: &'d mut UiData, scope: &Scope<'_>) -> Option<&'d mut UiData> {
    for &(key, index) in scope {
        data = &mut data.lists.get_mut(key)?.get_mut(index)?.data;
    }
    Some(data)
}

fn lookup<'a, V>(
    root: &'a UiData,
    scope: &Scope<'_>,
    field: impl Fn(&UiData) -> &BTreeMap<String, V>,
    key: &str,
) -> Option<&'a V> {
    if key.is_empty() {
        return None;
    }
    scoped_data(root, scope)
        .and_then(|data| field(data).get(key))
        .or_else(|| field(root).get(key))
}

fn write<V>(
    root: &mut UiData,
    scope: &Scope<'_>,
    field: impl for<'d> Fn(&'d UiData) -> &'d BTreeMap<String, V>,
    field_mut: impl for<'d> Fn(&'d mut UiData) -> &'d mut BTreeMap<String, V>,
    key: String,
    value: V,
) {
    // A local field wins; otherwise update the root field when it exists.
    // A new field is created in the current row. Private state is root scoped.
    let use_scope = !scope.is_empty()
        && (scoped_data(root, scope).is_some_and(|data| field(data).contains_key(&key))
            || !field(root).contains_key(&key));
    let target = if use_scope {
        scoped_data_mut(root, scope)
    } else {
        Some(root)
    };
    if let Some(data) = target {
        field_mut(data).insert(key, value);
    }
}

struct Renderer<'s, 'd> {
    graph: Graph<'s>,
    data: &'d mut UiData,
    styles: &'s BTreeMap<String, UiStyle>,
    textures: &'s UiTextures,
    screen_id: &'s str,
    id_prefix: egui::Id,
    scale: f32,
    interactive: bool,
    visits_left: usize,
    item_path: Vec<String>,
    events: Vec<UiEvent>,
}

struct PaintStyle<'a> {
    color: [u8; 4],
    text_color: [u8; 4],
    font_size: f32,
    rounding: f32,
    skin: &'a UiSkin,
}

impl<'a> PaintStyle<'a> {
    fn resolve(node: &'a UiNode, styles: &'a BTreeMap<String, UiStyle>) -> Self {
        if let Some(style) = styles.get(&node.style_key) {
            return Self {
                color: style.color,
                text_color: style.text_color,
                font_size: style.font_size,
                rounding: style.rounding,
                skin: &style.skin,
            };
        }
        Self {
            color: node.color,
            text_color: node.text_color,
            font_size: node.font_size,
            rounding: node.rounding,
            skin: &node.skin,
        }
    }

    fn rounding(&self, rect: egui::Rect, scale: f32) -> f32 {
        (self.rounding.clamp(0.0, MAX_COORD) * scale).min(rect.width().min(rect.height()) * 0.5)
    }

    fn border(&self, painter: &egui::Painter, rect: egui::Rect, scale: f32) {
        let width = (self.skin.border_width * scale).min(rect.width().min(rect.height()) * 0.5);
        if width.is_finite() && width > 0.0 && self.skin.border_color[3] > 0 {
            painter.rect_stroke(
                rect.shrink(width * 0.5),
                (self.rounding(rect, scale) - width * 0.5).max(0.0),
                egui::Stroke::new(width, rgba(self.skin.border_color)),
            );
        }
    }
}

impl<'s> Renderer<'s, '_> {
    fn draw_children(
        &mut self,
        ui: &mut egui::Ui,
        parent: Option<u64>,
        parent_rect: egui::Rect,
        clip: egui::Rect,
        scope: &Scope<'s>,
        parent_enabled: bool,
    ) {
        if !clip.is_positive() {
            return;
        }
        for child in 0..self.graph.children(parent).len() {
            if self.visits_left == 0 {
                break;
            }
            self.visits_left -= 1;
            let index = self.graph.children(parent)[child];
            let node = &self.graph.nodes[index];
            let visible = self.flag(scope, &node.visible_binding, node.visible);
            if !visible {
                continue;
            }
            let enabled = parent_enabled && self.flag(scope, &node.enabled_binding, node.enabled);
            let rect = node_rect(node, parent_rect, self.scale);
            if !rect.is_finite() {
                continue;
            }
            let clip = clip.intersect(rect);
            if !clip.is_positive() {
                continue;
            }
            let id = self.id_prefix.with((node.id, &self.item_path));
            let mut widget_ui = egui::Ui::new(ui.ctx().clone(), ui.layer_id(), id, rect, clip);
            widget_ui.set_style(ui.style().clone());
            widget_ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            let style = PaintStyle::resolve(node, self.styles);
            match node.kind {
                WidgetKind::Panel => {
                    self.background(widget_ui.painter(), rect, &style, scope, None, enabled);
                    self.draw_children(ui, Some(node.id), rect, clip, scope, enabled);
                    style.border(widget_ui.painter(), rect, self.scale);
                }
                WidgetKind::Repeater => {
                    self.background(widget_ui.painter(), rect, &style, scope, None, enabled);
                    self.repeater(&mut widget_ui, node, rect, scope, enabled);
                    style.border(widget_ui.painter(), rect, self.scale);
                }
                _ => self.widget(&mut widget_ui, node, rect, scope, enabled, &style),
            }
        }
    }

    fn flag(&self, scope: &Scope<'_>, key: &str, fallback: bool) -> bool {
        if self.interactive {
            lookup(self.data, scope, |d| &d.flags, key)
                .copied()
                .unwrap_or(fallback)
        } else {
            fallback
        }
    }

    fn background(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        style: &PaintStyle<'_>,
        scope: &Scope<'_>,
        response: Option<&egui::Response>,
        enabled: bool,
    ) {
        let skin = style.skin;
        let pressed = response.is_some_and(egui::Response::is_pointer_button_down_on);
        let hovered = response.is_some_and(|response| response.hovered() || response.has_focus());
        let override_color = if pressed {
            skin.pressed_color.or(skin.hover_color)
        } else if hovered {
            skin.hover_color
        } else {
            None
        };
        let mut fill = override_color
            .map(rgba)
            .unwrap_or_else(|| highlighted(rgba(style.color), response));
        if !enabled {
            fill = fill.gamma_multiply(0.58);
        }
        if fill.a() > 0 {
            painter.rect_filled(rect, style.rounding(rect, self.scale), fill);
        }

        let bound = self
            .interactive
            .then(|| lookup(self.data, scope, |d| &d.text, &skin.image_binding))
            .flatten()
            .map(String::as_str)
            .unwrap_or_default();
        let texture = [
            if pressed {
                skin.pressed_texture.as_str()
            } else {
                ""
            },
            if pressed || hovered {
                skin.hover_texture.as_str()
            } else {
                ""
            },
            bound,
            skin.texture.as_str(),
        ]
        .into_iter()
        .find_map(|key| self.textures.entries.get(key));
        if let Some(texture) = texture.filter(|_| skin.valid_geometry()) {
            painter.add(sprite_mesh(texture, rect, skin, self.scale));
        }
    }

    fn repeater(
        &mut self,
        ui: &mut egui::Ui,
        node: &'s UiNode,
        rect: egui::Rect,
        scope: &Scope<'s>,
        enabled: bool,
    ) {
        let row_height = node.row_height * self.scale;
        if !row_height.is_finite() || row_height <= 0.0 {
            return;
        }
        if !self.interactive {
            let row = row_rect(rect, 0.0, row_height);
            self.draw_children(
                ui,
                Some(node.id),
                row,
                ui.clip_rect().intersect(row),
                scope,
                enabled,
            );
            return;
        }

        let key = node.list_binding.as_str();
        if key.is_empty() || self.graph.children(Some(node.id)).is_empty() {
            return;
        }
        let list_scope =
            if scoped_data(self.data, scope).is_some_and(|data| data.lists.contains_key(key)) {
                scope
            } else {
                &[]
            };
        let Some(rows) = scoped_data(self.data, list_scope).and_then(|data| data.lists.get(key))
        else {
            return;
        };
        if rows.is_empty() || rows.len() > MAX_ROWS {
            return;
        }

        // Never show ambiguous rows. Skip every copy of a repeated ID, as
        // well as blank/control-character IDs, before assigning row positions.
        let mut counts = BTreeMap::new();
        for item in rows {
            if valid_key(&item.id) {
                *counts.entry(item.id.as_str()).or_insert(0) += 1;
            }
        }
        let rows: Vec<_> = rows
            .iter()
            .enumerate()
            .filter(|(_, item)| counts.get(item.id.as_str()) == Some(&1))
            .map(|(index, item)| (index, item.id.clone()))
            .collect();
        if rows.is_empty() {
            return;
        }

        let stride = row_height + node.gap * self.scale;
        let height = (rows.len() - 1) as f32 * stride + row_height;
        if !height.is_finite() {
            return;
        }
        let clip = ui.clip_rect();
        let mut draw_rows =
            |content: &mut egui::Ui, row_parent: egui::Rect, first: usize, end: usize| {
                for (display_index, (source_index, id)) in
                    rows.iter().enumerate().take(end).skip(first)
                {
                    if self.visits_left == 0 {
                        break;
                    }
                    self.visits_left -= 1;
                    let row = row_rect(row_parent, display_index as f32 * stride, row_height);
                    let mut row_scope = list_scope.to_vec();
                    row_scope.push((key, *source_index));
                    self.item_path.push(id.clone());
                    self.draw_children(
                        content,
                        Some(node.id),
                        row,
                        clip.intersect(content.clip_rect()).intersect(row),
                        &row_scope,
                        enabled,
                    );
                    self.item_path.pop();
                }
            };
        if height <= rect.height() {
            draw_rows(ui, rect, 0, rows.len());
        } else {
            egui::ScrollArea::vertical()
                .id_source("rows")
                .min_scrolled_height(0.0)
                .max_height(rect.height())
                .max_width(rect.width())
                .auto_shrink([false, false])
                .drag_to_scroll(false)
                .enable_scrolling(enabled)
                .show_viewport(ui, |content, viewport| {
                    content.set_min_size(egui::vec2(viewport.width(), height));
                    let row_parent = egui::Rect::from_min_size(
                        content.max_rect().min,
                        egui::vec2(viewport.width(), height),
                    );
                    let first = (viewport.min.y / stride).floor() as usize;
                    let end = (viewport.max.y / stride).ceil() as usize;
                    draw_rows(content, row_parent, first, end);
                });
        }
    }

    fn widget(
        &mut self,
        ui: &mut egui::Ui,
        node: &UiNode,
        rect: egui::Rect,
        scope: &Scope<'_>,
        enabled: bool,
        style: &PaintStyle<'_>,
    ) {
        let interactive = self.interactive && enabled;
        let id = ui.id().with("input");
        let mut text_color = rgba(style.text_color);
        if !enabled {
            text_color = text_color.gamma_multiply(0.55);
        }
        let family = match style.skin.font {
            UiFont::Proportional => egui::FontFamily::Proportional,
            UiFont::Monospace => egui::FontFamily::Monospace,
        };
        let font_size = if positive_size(style.font_size) {
            style.font_size
        } else {
            node.font_size
        };
        let font = egui::FontId::new((font_size * self.scale).clamp(1.0, 512.0), family);
        let padding = (style.skin.padding.clamp(0.0, MAX_COORD) * self.scale)
            .min(rect.width() * 0.25)
            .min(rect.height() * 0.20);
        let inner = rect.shrink(padding);
        let rounding = style.rounding(rect, self.scale);
        let align = style.skin.align;
        let painter = ui.painter().clone();

        let key = if node.binding.is_empty() {
            // Length prefixes distinguish arbitrary item IDs, even when a
            // nested list happens to reuse the same physical root records.
            let mut key = format!(
                "\0enigma-ui:{}:{}:{}",
                self.screen_id.len(),
                self.screen_id,
                node.id
            );
            for item in &self.item_path {
                use std::fmt::Write as _;
                let _ = write!(key, ":{}:{}", item.len(), item);
            }
            key
        } else {
            node.binding.clone()
        };
        let value_scope = if node.binding.is_empty() { &[] } else { scope };
        let binding = if self.interactive { key.as_str() } else { "" };
        let text = lookup(self.data, value_scope, |d| &d.text, binding)
            .map(String::as_str)
            .unwrap_or(&node.text);
        let number = |data: &UiData| {
            lookup(data, value_scope, |d| &d.numbers, binding)
                .copied()
                .filter(|v| v.is_finite())
                .unwrap_or(node.value)
        };
        let event = match node.kind {
            WidgetKind::Label => {
                self.background(&painter, rect, style, scope, None, enabled);
                paint_text(&painter, inner, text, font, text_color, align, true);
                None
            }
            WidgetKind::Button => {
                let response = interactive.then(|| ui.interact(rect, id, egui::Sense::click()));
                self.background(&painter, rect, style, scope, response.as_ref(), enabled);
                paint_text(&painter, inner, text, font, text_color, align, false);
                response
                    .filter(egui::Response::clicked)
                    .map(|_| UiEventValue::Click)
            }
            WidgetKind::Toggle => {
                let mut checked = lookup(self.data, value_scope, |d| &d.flags, binding)
                    .copied()
                    .unwrap_or(node.checked);
                let response = interactive.then(|| ui.interact(rect, id, egui::Sense::click()));
                let clicked = response.as_ref().is_some_and(egui::Response::clicked);
                if clicked {
                    checked = !checked;
                }
                self.background(&painter, rect, style, scope, response.as_ref(), enabled);
                let side = inner.height().min(inner.width()).min(23.0 * self.scale);
                let mark = egui::Rect::from_center_size(
                    egui::pos2(inner.left() + side * 0.5, inner.center().y),
                    egui::Vec2::splat(side),
                );
                painter.rect_stroke(
                    mark,
                    side * 0.22,
                    egui::Stroke::new((side * 0.10).max(1.0), text_color),
                );
                if checked {
                    painter.rect_filled(mark.shrink(side * 0.21), side * 0.14, text_color);
                }
                let label = egui::Rect::from_min_max(
                    egui::pos2(mark.right() + padding, inner.top()),
                    inner.max,
                );
                paint_text(&painter, label, &node.text, font, text_color, align, false);
                clicked.then_some(UiEventValue::Bool(checked))
            }
            WidgetKind::Slider | WidgetKind::Progress => {
                let [min, max] = node.range;
                let mut value = number(self.data).clamp(min, max);
                let mut response = None;
                let mut changed = false;
                if node.kind == WidgetKind::Slider && interactive {
                    let input = ui.interact(rect, id, egui::Sense::click_and_drag());
                    if input.dragged() || input.clicked() {
                        if let Some(pos) = input.interact_pointer_pos() {
                            let ratio = ((pos.x - inner.left()) / inner.width()).clamp(0.0, 1.0);
                            let next = min + ratio * (max - min);
                            changed = next != value;
                            value = next;
                        }
                    } else if input.has_focus() {
                        let direction = ui.input(|input| {
                            i32::from(input.key_pressed(egui::Key::ArrowRight))
                                - i32::from(input.key_pressed(egui::Key::ArrowLeft))
                        });
                        let next = (value + direction as f32 * (max - min) / 100.0).clamp(min, max);
                        changed = next != value;
                        value = next;
                    }
                    response = Some(input);
                }
                let ratio = (value - min) / (max - min);
                self.background(&painter, rect, style, scope, response.as_ref(), enabled);
                let bar = egui::Rect::from_min_max(
                    inner.min,
                    egui::pos2(inner.left() + ratio * inner.width(), inner.bottom()),
                );
                if bar.is_positive() {
                    painter.rect_filled(
                        bar,
                        rounding.min(bar.width() * 0.5),
                        text_color.gamma_multiply(0.66),
                    );
                }
                if node.kind == WidgetKind::Slider {
                    painter.circle_filled(
                        egui::pos2(bar.right(), rect.center().y),
                        inner.height().min(rect.width()) * 0.28,
                        text_color,
                    );
                }
                changed.then_some(UiEventValue::Number(value))
            }
            WidgetKind::TextInput => {
                // Read pointer state for the background without claiming input
                // from the text editor that is painted above it.
                let art_response = interactive.then(|| {
                    let mut response = ui.interact(rect, id.with("art"), egui::Sense::hover());
                    response.is_pointer_button_down_on = response.hovered()
                        && ui.input(|input| {
                            input.pointer.primary_down()
                                && input
                                    .pointer
                                    .press_origin()
                                    .is_some_and(|pos| ui.clip_rect().contains(pos))
                        });
                    response
                });
                self.background(&painter, rect, style, scope, art_response.as_ref(), enabled);
                if interactive {
                    let mut value = limited_text(text).to_owned();
                    let mut editor =
                        egui::Ui::new(ui.ctx().clone(), ui.layer_id(), id, rect, ui.clip_rect());
                    editor.set_style(ui.style().clone());
                    let margin = egui::vec2(padding, ((rect.height() - font.size) * 0.5).max(0.0));
                    let response = editor.add(
                        egui::TextEdit::singleline(&mut value)
                            .id(id)
                            .font(font)
                            .text_color(text_color)
                            .frame(false)
                            .margin(margin)
                            .desired_width(inner.width())
                            .horizontal_align(text_align(align))
                            .char_limit(MAX_TEXT / 4),
                    );
                    response.changed().then_some(UiEventValue::Text(value))
                } else {
                    paint_text(&painter, inner, text, font, text_color, align, false);
                    None
                }
            }
            WidgetKind::Image => {
                self.background(&painter, rect, style, scope, None, enabled);
                None
            }
            WidgetKind::Panel | WidgetKind::Repeater => unreachable!(),
        };
        style.border(&painter, rect, self.scale);
        if let Some(value) = event {
            match &value {
                UiEventValue::Click => {}
                UiEventValue::Bool(value) => write(
                    self.data,
                    value_scope,
                    |d| &d.flags,
                    |d| &mut d.flags,
                    key,
                    *value,
                ),
                UiEventValue::Number(value) => write(
                    self.data,
                    value_scope,
                    |d| &d.numbers,
                    |d| &mut d.numbers,
                    key,
                    *value,
                ),
                UiEventValue::Text(value) => write(
                    self.data,
                    value_scope,
                    |d| &d.text,
                    |d| &mut d.text,
                    key,
                    value.clone(),
                ),
            }
            if !node.event.is_empty() {
                self.events.push(UiEvent {
                    name: node.event.clone(),
                    widget: node.name.clone(),
                    item_id: self.item_path.last().cloned(),
                    item_path: self.item_path.clone(),
                    value,
                });
            }
        }
    }
}

fn rgba([r, g, b, a]: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

fn highlighted(color: egui::Color32, response: Option<&egui::Response>) -> egui::Color32 {
    if response.is_some_and(egui::Response::is_pointer_button_down_on) {
        color.gamma_multiply(0.83)
    } else if response.is_some_and(|response| response.hovered() || response.has_focus()) {
        let [r, g, b, a] = color.to_array();
        let lighten = |channel| channel + (a.saturating_sub(channel) / 8);
        egui::Color32::from_rgba_premultiplied(lighten(r), lighten(g), lighten(b), a)
    } else {
        color
    }
}

fn limited_text(text: &str) -> &str {
    let mut end = text.len().min(MAX_TEXT);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn text_align(align: UiTextAlign) -> egui::Align {
    match align {
        UiTextAlign::Left => egui::Align::Min,
        UiTextAlign::Center => egui::Align::Center,
        UiTextAlign::Right => egui::Align::Max,
    }
}

fn paint_text(
    painter: &egui::Painter,
    rect: egui::Rect,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
    align: UiTextAlign,
    top: bool,
) {
    if !rect.is_positive() || text.is_empty() {
        return;
    }
    let painter = painter.with_clip_rect(rect);
    let mut job =
        egui::text::LayoutJob::simple(limited_text(text).to_owned(), font, color, rect.width());
    job.halign = text_align(align);
    let galley = painter.fonts(|fonts| fonts.layout_job(job));
    let x = match align {
        UiTextAlign::Left => rect.left(),
        UiTextAlign::Center => rect.center().x,
        UiTextAlign::Right => rect.right(),
    };
    let y = if top {
        rect.top()
    } else {
        rect.top() + (rect.height() - galley.size().y).max(0.0) * 0.5
    };
    painter.galley(egui::pos2(x, y), galley);
}

fn sprite_mesh(texture: &UiTexture, mut rect: egui::Rect, skin: &UiSkin, scale: f32) -> egui::Mesh {
    let texture_id = if skin.pixelated {
        texture.pixelated.id()
    } else {
        texture.smooth.id()
    };
    let mut mesh = egui::Mesh::with_texture(texture_id);
    let [u0, v0, u1, v1] = skin.uv;
    let mut uv = egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));
    let texture_size = egui::vec2(texture.size[0] as f32, texture.size[1] as f32);
    let source = uv.size() * texture_size;
    let tint = rgba(skin.tint);
    match skin.fit {
        ImageFit::Stretch => mesh.add_rect_with_uv(rect, uv, tint),
        ImageFit::Contain => {
            let factor = (rect.width() / source.x).min(rect.height() / source.y);
            rect = egui::Rect::from_center_size(rect.center(), source * factor);
            mesh.add_rect_with_uv(rect, uv, tint);
        }
        ImageFit::Cover => {
            let factor = (rect.width() / source.x).max(rect.height() / source.y);
            let crop = (rect.size() / factor / texture_size).min(uv.size());
            uv = egui::Rect::from_center_size(uv.center(), crop).intersect(uv);
            mesh.add_rect_with_uv(rect, uv, tint);
        }
        ImageFit::NineSlice => {
            let [left, top, right, bottom] = skin.nine_slice;
            let (x, u) = slice_axis(
                rect.x_range(),
                uv.x_range(),
                texture_size.x,
                left,
                right,
                scale,
            );
            let (y, v) = slice_axis(
                rect.y_range(),
                uv.y_range(),
                texture_size.y,
                top,
                bottom,
                scale,
            );
            for row in 0..3 {
                for column in 0..3 {
                    let dst = egui::Rect::from_min_max(
                        egui::pos2(x[column], y[row]),
                        egui::pos2(x[column + 1], y[row + 1]),
                    );
                    if dst.is_positive() {
                        let src = egui::Rect::from_min_max(
                            egui::pos2(u[column], v[row]),
                            egui::pos2(u[column + 1], v[row + 1]),
                        );
                        mesh.add_rect_with_uv(dst, src, tint);
                    }
                }
            }
        }
    }
    mesh
}

fn slice_axis(
    dst: egui::emath::Rangef,
    src: egui::emath::Rangef,
    texture_side: f32,
    start: f32,
    end: f32,
    scale: f32,
) -> ([f32; 4], [f32; 4]) {
    let (d0, d1) = (dst.min, dst.max);
    let (s0, s1) = (src.min, src.max);
    let source_factor = ((s1 - s0) * texture_side / (start + end)).min(1.0);
    let (start, end) = (start * source_factor, end * source_factor);
    let dest_factor = ((d1 - d0) / (start + end)).min(scale);
    let d_start = (d0 + start * dest_factor).clamp(d0, d1);
    let d_end = (d1 - end * dest_factor).clamp(d_start, d1);
    let s_start = (s0 + start / texture_side).clamp(s0, s1);
    let s_end = (s1 - end / texture_side).clamp(s_start, s1);
    ([d0, d_start, d_end, d1], [s0, s_start, s_end, s1])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0))
    }

    #[test]
    fn dialogue_speaker_fits_within_its_label_height() {
        let ctx = egui::Context::default();
        let screen = UiScreen::dialogue("dialogue");
        let mut data = screen.sample_data.clone();
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            },
            |ctx| {
                let mut ui = egui::Ui::new(
                    ctx.clone(),
                    egui::LayerId::background(),
                    egui::Id::new("test"),
                    rect,
                    rect,
                );
                draw_screen(&mut ui, &screen, &mut data, rect, true);
            },
        );
        let (clip, ink) = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "Warden Iona" => {
                    Some((shape.clip_rect, text.visual_bounding_rect()))
                }
                _ => None,
            })
            .expect("speaker label should produce text");
        assert!(
            clip.contains_rect(ink),
            "speaker text {ink:?} is clipped by {clip:?}"
        );
    }

    #[test]
    fn old_layouts_default_to_visible_enabled_literal_widgets() {
        let screen: UiScreen = serde_json::from_str(
            r#"{"id":"test","name":"Test","size":[400,300],"nodes":[{"id":1,"name":"Title","kind":"Label"}]}"#,
        ).unwrap();
        screen.validate().unwrap();
        assert!(screen.nodes[0].visible && screen.nodes[0].enabled);
        assert!(screen.nodes[0].binding.is_empty());
        assert_eq!(screen.sample_data, UiData::default());
        let json = serde_json::to_string(&screen).unwrap();
        assert_eq!(serde_json::from_str::<UiScreen>(&json).unwrap(), screen);
    }

    #[test]
    fn cycles_are_rejected_and_cannot_hang_layout() {
        let mut screen = UiScreen::new("Example", "example");
        let mut one = UiNode::new(1, WidgetKind::Panel, "One");
        let mut two = UiNode::new(2, WidgetKind::Panel, "Two");
        one.parent = Some(2);
        two.parent = Some(1);
        screen.nodes = vec![one, two];
        assert!(screen.validate().is_err());
        assert!(layout_nodes(&screen, bounds()).is_empty());
    }

    #[test]
    fn child_anchor_uses_parent_and_widget_pivots_after_letterboxing() {
        let mut screen = UiScreen::new("Example", "example");
        screen.size = [200.0, 100.0];
        let mut panel = UiNode::new(1, WidgetKind::Panel, "Panel");
        panel.size = [100.0, 50.0];
        panel.position = [10.0, 5.0];
        let mut button = UiNode::new(2, WidgetKind::Button, "Button");
        button.parent = Some(1);
        button.anchor = [1.0, 1.0];
        button.size = [30.0, 10.0];
        button.position = [-4.0, -3.0];
        screen.nodes = vec![button, panel];
        let layout = layout_nodes(&screen, bounds());
        assert_eq!(
            layout[0],
            (
                1,
                egui::Rect::from_min_max(egui::pos2(20.0, 60.0), egui::pos2(220.0, 160.0))
            )
        );
        assert_eq!(
            layout[1],
            (
                2,
                egui::Rect::from_min_max(egui::pos2(152.0, 134.0), egui::pos2(212.0, 154.0))
            )
        );
    }

    fn frame(
        ctx: &egui::Context,
        screen: &UiScreen,
        data: &mut UiData,
        events: Vec<egui::Event>,
        interactive: bool,
    ) -> Vec<UiEvent> {
        let mut result = Vec::new();
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(bounds()),
                events,
                ..Default::default()
            },
            |ctx| {
                let mut ui = egui::Ui::new(
                    ctx.clone(),
                    egui::LayerId::background(),
                    egui::Id::new("test"),
                    bounds(),
                    bounds(),
                );
                result = draw_screen(&mut ui, screen, data, bounds(), interactive);
            },
        );
        result
    }

    fn click(
        ctx: &egui::Context,
        screen: &UiScreen,
        data: &mut UiData,
        pos: egui::Pos2,
        interactive: bool,
    ) -> Vec<UiEvent> {
        let event = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        assert!(frame(
            ctx,
            screen,
            data,
            vec![egui::Event::PointerMoved(pos), event(true)],
            interactive
        )
        .is_empty());
        frame(ctx, screen, data, vec![event(false)], interactive)
    }

    fn choice_screen() -> UiScreen {
        let mut screen = UiScreen::new("Choices", "choices");
        screen.size = [400.0, 300.0];
        let mut list = UiNode::new(1, WidgetKind::Repeater, "List");
        list.list_binding = "choices".into();
        list.size = [300.0, 120.0];
        list.row_height = 40.0;
        list.gap = 0.0;
        let mut button = UiNode::new(2, WidgetKind::Button, "Choice");
        button.parent = Some(1);
        button.size = [200.0, 35.0];
        button.binding = "label".into();
        button.enabled_binding = "available".into();
        button.event = "choose".into();
        screen.nodes = vec![list, button];
        screen
    }

    #[test]
    fn choices_emit_stable_ids_after_reordering_and_ignore_duplicates() {
        let ctx = egui::Context::default();
        let screen = choice_screen();
        let mut data = UiData::default();
        data.set_text("label", "Shared label");
        let point = egui::pos2(20.0, 15.0);
        frame(&ctx, &screen, &mut data, vec![], true);
        assert!(click(&ctx, &screen, &mut data, point, true).is_empty());
        let yes = UiItem::new("yes");
        let no = UiItem::new("no").with_bool("available", false);
        data.set_items("choices", vec![yes.clone(), no.clone()]);
        frame(&ctx, &screen, &mut data, vec![], true);
        assert_eq!(
            click(&ctx, &screen, &mut data, point, true)[0].item_path,
            vec!["yes"]
        );
        data.set_items("choices", vec![no, yes.clone()]);
        frame(&ctx, &screen, &mut data, vec![], true);
        assert!(click(&ctx, &screen, &mut data, point, true).is_empty());
        let events = click(&ctx, &screen, &mut data, egui::pos2(20.0, 55.0), true);
        assert_eq!(
            events,
            vec![UiEvent {
                name: "choose".into(),
                widget: "Choice".into(),
                item_id: Some("yes".into()),
                item_path: vec!["yes".into()],
                value: UiEventValue::Click
            }]
        );
        data.set_items("choices", vec![yes.clone(), yes]);
        frame(&ctx, &screen, &mut data, vec![], true);
        assert!(click(&ctx, &screen, &mut data, point, true).is_empty());
    }

    #[test]
    fn row_controls_write_existing_root_bindings_without_changing_sample_defaults() {
        let ctx = egui::Context::default();
        let mut screen = choice_screen();
        screen.nodes[1].kind = WidgetKind::Toggle;
        screen.nodes[1].binding = "selected".into();
        screen.nodes[1].event = "toggle".into();
        screen.sample_data.set_bool("selected", false);
        screen
            .sample_data
            .set_items("choices", vec![UiItem::new("row")]);
        let mut data = screen.sample_data.clone();
        frame(&ctx, &screen, &mut data, vec![], true);
        let point = egui::pos2(20.0, 15.0);
        let events = click(&ctx, &screen, &mut data, point, true);
        assert_eq!(data.flags["selected"], true);
        assert_eq!(screen.sample_data.flags["selected"], false);
        assert_eq!(events[0].item_id.as_deref(), Some("row"));
        assert_eq!(events[0].value, UiEventValue::Bool(true));
        assert!(click(&ctx, &screen, &mut data, point, false).is_empty());
        assert_eq!(data.flags["selected"], true);
    }

    #[test]
    fn text_focus_follows_item_identity_when_rows_move() {
        let ctx = egui::Context::default();
        let mut screen = choice_screen();
        screen.nodes[1].kind = WidgetKind::TextInput;
        screen.nodes[1].binding.clear();
        screen.nodes[1].text.clear();
        let mut data = UiData::default();
        let a = UiItem::new("a");
        let b = UiItem::new("b");
        data.set_items("choices", vec![a.clone(), b.clone()]);
        frame(&ctx, &screen, &mut data, vec![], true);
        click(&ctx, &screen, &mut data, egui::pos2(20.0, 15.0), true);
        let first = frame(
            &ctx,
            &screen,
            &mut data,
            vec![egui::Event::Text("one".into())],
            true,
        );
        assert_eq!(first[0].value, UiEventValue::Text("one".into()));
        let mut rows = data.lists.remove("choices").unwrap();
        rows.reverse();
        data.set_items("choices", rows);
        let next = frame(
            &ctx,
            &screen,
            &mut data,
            vec![egui::Event::Text(" two".into())],
            true,
        );
        assert_eq!(next[0].item_id.as_deref(), Some("a"));
        assert_eq!(next[0].value, UiEventValue::Text("one two".into()));
    }

    #[test]
    fn choice_scrolls_to_rows_beyond_the_viewport() {
        let ctx = egui::Context::default();
        let screen = choice_screen();
        let mut data = UiData::default();
        data.set_items(
            "choices",
            (0..6).map(|i| UiItem::new(format!("row-{i}"))).collect(),
        );
        frame(&ctx, &screen, &mut data, vec![], true);
        assert!(click(&ctx, &screen, &mut data, egui::pos2(20.0, 135.0), true).is_empty());
        frame(
            &ctx,
            &screen,
            &mut data,
            vec![
                egui::Event::PointerMoved(egui::pos2(20.0, 95.0)),
                egui::Event::Scroll(egui::vec2(0.0, -200.0)),
            ],
            true,
        );
        frame(&ctx, &screen, &mut data, vec![], true);
        let events = click(&ctx, &screen, &mut data, egui::pos2(20.0, 95.0), true);
        assert_eq!(events[0].item_id.as_deref(), Some("row-5"));
    }

    #[test]
    fn adding_scrollbar_preserves_focus_of_existing_row() {
        let ctx = egui::Context::default();
        let mut screen = choice_screen();
        screen.nodes[1].kind = WidgetKind::TextInput;
        screen.nodes[1].binding = "label".into();
        screen.nodes[1].text.clear();
        let mut data = UiData::default();
        data.set_items("choices", vec![UiItem::new("a").with_text("label", "")]);
        frame(&ctx, &screen, &mut data, vec![], true);
        click(&ctx, &screen, &mut data, egui::pos2(20.0, 15.0), true);
        frame(
            &ctx,
            &screen,
            &mut data,
            vec![egui::Event::Text("first".into())],
            true,
        );
        data.lists
            .get_mut("choices")
            .unwrap()
            .extend((0..4).map(|i| UiItem::new(format!("new-{i}"))));
        let events = frame(
            &ctx,
            &screen,
            &mut data,
            vec![egui::Event::Text(" second".into())],
            true,
        );
        assert_eq!(events[0].item_id.as_deref(), Some("a"));
        assert_eq!(events[0].value, UiEventValue::Text("first second".into()));
        assert_eq!(data.lists["choices"][0].data.text["label"], "first second");
        assert!(!data.text.contains_key("label"));
    }

    #[test]
    fn nested_row_identity_is_preserved_when_inner_list_falls_back_to_root() {
        let ctx = egui::Context::default();
        let mut screen = choice_screen();
        let mut inner = UiNode::new(3, WidgetKind::Repeater, "Inner");
        inner.parent = Some(1);
        inner.list_binding = "answers".into();
        inner.row_height = 34.0;
        inner.size = [250.0, 36.0];
        screen.nodes[1].parent = Some(3);
        screen.nodes.push(inner);
        let mut data = UiData::default();
        data.set_items(
            "choices",
            vec![UiItem::new("outer-1"), UiItem::new("outer-2")],
        );
        data.set_items("answers", vec![UiItem::new("inner-1")]);
        frame(&ctx, &screen, &mut data, vec![], true);
        let events = click(&ctx, &screen, &mut data, egui::pos2(20.0, 55.0), true);
        assert_eq!(events[0].item_path, ["outer-2", "inner-1"]);
        assert_eq!(events[0].item_id.as_deref(), Some("inner-1"));
    }
    #[test]
    fn named_style_and_sprite_region_round_trip() {
        let mut screen = UiScreen::new("Art", "art");
        screen
            .textures
            .insert("sheet".into(), "ui/tiles.png".into());
        let mut node = UiNode::new(1, WidgetKind::Image, "Tile");
        node.style_key = "item".into();
        let mut style = UiStyle::default();
        style.skin.texture = "sheet".into();
        style.skin.uv = [0.5, 0.0, 1.0, 0.5];
        style.skin.pixelated = true;
        screen.styles.insert("item".into(), style);
        screen.nodes.push(node);
        screen.validate().unwrap();
        let json = serde_json::to_string(&screen).unwrap();
        assert_eq!(serde_json::from_str::<UiScreen>(&json).unwrap(), screen);
        screen.styles.clear();
        assert!(screen.validate().is_err());
    }

    fn art_frame(
        ctx: &egui::Context,
        screen: &UiScreen,
        data: &mut UiData,
        textures: &UiTextures,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(bounds()),
                events,
                ..Default::default()
            },
            |ctx| {
                let mut ui = egui::Ui::new(
                    ctx.clone(),
                    egui::LayerId::background(),
                    egui::Id::new("art"),
                    bounds(),
                    bounds(),
                );
                draw_screen_with_textures(&mut ui, screen, data, bounds(), true, textures);
            },
        )
    }

    fn art_meshes(output: &egui::FullOutput) -> impl Iterator<Item = &egui::Mesh> {
        output.shapes.iter().filter_map(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) => Some(mesh),
            _ => None,
        })
    }

    #[test]
    fn sharing_and_detaching_styles_preserves_label_and_repeater_fills() {
        let ctx = egui::Context::default();
        let fill_count = |screen: &UiScreen| {
            art_frame(
                &ctx,
                screen,
                &mut UiData::default(),
                &UiTextures::default(),
                vec![],
            )
            .shapes
            .iter()
            .filter(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill.a() > 0))
            .count()
        };
        for kind in [WidgetKind::Label, WidgetKind::Repeater] {
            let mut new = UiScreen::new("Test", "new");
            new.nodes.push(UiNode::new(1, kind, "Widget"));
            let legacy: UiScreen = serde_json::from_value(serde_json::json!({
                "id": "legacy", "name": "Test", "size": [400, 300], "nodes": [
                    {"id": 1, "name": "Widget", "kind": kind, "color": [44, 56, 75, 255]}
                ]
            }))
            .unwrap();
            for mut screen in [new, legacy] {
                assert_eq!(fill_count(&screen), 0);
                screen
                    .styles
                    .insert("shared".into(), UiStyle::from_node(&screen.nodes[0]));
                screen.nodes[0].style_key = "shared".into();
                assert_eq!(fill_count(&screen), 0, "sharing a transparent fill");

                screen.styles.get_mut("shared").unwrap().color = [44, 56, 75, 255];
                assert_eq!(fill_count(&screen), 1);
                screen.nodes[0].color = screen.styles["shared"].color;
                screen.nodes[0].style_key.clear();
                assert_eq!(fill_count(&screen), 1, "detaching an opaque fill");

                let reloaded =
                    serde_json::from_str(&serde_json::to_string(&screen).unwrap()).unwrap();
                assert_eq!(fill_count(&reloaded), 1, "reopening an opaque local fill");
            }
        }
    }

    #[test]
    fn nine_slice_mesh_respects_sprite_region_tint_and_nearest_sampling() {
        let ctx = egui::Context::default();
        let mut textures = UiTextures::default();
        assert!(textures
            .insert_rgba(&ctx, "sheet", [8, 8], &[0; 255])
            .is_err());
        textures
            .insert_rgba(&ctx, "sheet", [8, 8], &[255; 256])
            .unwrap();
        let mut screen = UiScreen::new("Art", "art");
        screen.size = [400.0, 300.0];
        let mut tile = UiNode::new(1, WidgetKind::Image, "Tile");
        tile.position = [10.0, 20.0];
        tile.size = [12.0, 8.0];
        tile.skin.texture = "sheet".into();
        tile.skin.fit = ImageFit::NineSlice;
        tile.skin.uv = [0.5, 0.0, 1.0, 0.5];
        tile.skin.nine_slice = [1.0; 4];
        tile.skin.tint = [80, 150, 220, 180];
        tile.skin.pixelated = true;
        screen.nodes.push(tile);
        let frame = art_frame(&ctx, &screen, &mut UiData::default(), &textures, vec![]);
        let mesh = art_meshes(&frame)
            .next()
            .expect("sprite should paint a mesh");
        assert!(frame
            .textures_delta
            .set
            .iter()
            .any(|(id, delta)| *id == mesh.texture_id
                && delta.options == egui::TextureOptions::NEAREST));
        assert_eq!(
            mesh.calc_bounds(),
            egui::Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(22.0, 28.0))
        );
        let sprite = egui::Rect::from_min_max(egui::pos2(0.5, 0.0), egui::pos2(1.0, 0.5));
        assert!(mesh
            .vertices
            .iter()
            .all(|v| sprite.contains(v.uv) && v.color == rgba([80, 150, 220, 180])));
        assert!(mesh
            .vertices
            .iter()
            .any(|v| v.pos == egui::pos2(11.0, 21.0) && v.uv == egui::pos2(0.625, 0.125)));
    }

    #[test]
    fn styled_row_image_uses_binding_then_hover_and_pressed_artwork() {
        for kind in [WidgetKind::Button, WidgetKind::TextInput] {
            let ctx = egui::Context::default();
            let mut textures = UiTextures::default();
            for key in ["normal", "row", "hover", "pressed"] {
                textures.insert_rgba(&ctx, key, [1, 1], &[255; 4]).unwrap();
            }
            let mut screen = choice_screen();
            screen.nodes[1].kind = kind;
            let mut style = UiStyle::default();
            style.skin.texture = "normal".into();
            style.skin.hover_texture = "hover".into();
            style.skin.pressed_texture = "pressed".into();
            style.skin.image_binding = "art".into();
            screen.styles.insert("choice".into(), style);
            screen.nodes[1].style_key = "choice".into();
            let mut data = UiData::default();
            data.set_items(
                "choices",
                vec![
                    UiItem::new("one").with_text("art", "row"),
                    UiItem::new("two").with_text("art", "unknown"),
                ],
            );
            let frame = art_frame(&ctx, &screen, &mut data, &textures, vec![]);
            let ids: Vec<_> = art_meshes(&frame).map(|mesh| mesh.texture_id).collect();
            assert_eq!(
                ids,
                [
                    textures.entries["row"].smooth.id(),
                    textures.entries["normal"].smooth.id()
                ]
            );
            let pos = egui::pos2(20.0, 15.0);
            let hover = art_frame(
                &ctx,
                &screen,
                &mut data,
                &textures,
                vec![egui::Event::PointerMoved(pos)],
            );
            assert_eq!(
                art_meshes(&hover).next().unwrap().texture_id,
                textures.entries["hover"].smooth.id()
            );
            let press = art_frame(
                &ctx,
                &screen,
                &mut data,
                &textures,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert_eq!(
                art_meshes(&press).next().unwrap().texture_id,
                textures.entries["pressed"].smooth.id()
            );
        }
    }
}
