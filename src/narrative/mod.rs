//! Portable narrative data and evaluation. No renderer or editor dependency.
//! Attributes use `character_id.attribute` keys; world attributes are unqualified.
//! All conditions on a choice must pass. Events are returned to the game, never executed here.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

type Extra = BTreeMap<String, serde_json::Value>;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StoryNode {
    pub id: String,
    #[serde(default)]
    pub speaker: String,
    pub text: String,
    /// Optional project-resource-relative narration clip. Empty means silent.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub audio: String,
    #[serde(default)]
    pub choices: Vec<Choice>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub character: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub portrait: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on_enter: Vec<String>,
    #[serde(default, flatten)]
    pub extra: Extra,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub text: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grants: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
    #[serde(default, flatten)]
    pub extra: Extra,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CharacterLibrary {
    #[serde(default)]
    pub characters: Vec<Character>,
    #[serde(default, flatten)]
    pub extra: Extra,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Variant name -> resource-relative image path.
    #[serde(default)]
    pub portraits: BTreeMap<String, String>,
    #[serde(default)]
    pub models: BTreeMap<String, ModelAsset>,
    #[serde(default)]
    pub attributes: BTreeMap<String, i64>,
    #[serde(default, flatten)]
    pub extra: Extra,
}

/// A project-resource-relative glTF/GLB model, independent of the 2D artwork.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelAsset {
    pub path: String,
    #[serde(default = "unit_scale")]
    pub scale: f32,
    #[serde(default)]
    pub animation: String,
    #[serde(default, flatten)]
    pub extra: Extra,
}
fn unit_scale() -> f32 {
    1.0
}
impl Default for ModelAsset {
    fn default() -> Self {
        Self {
            path: String::new(),
            scale: 1.0,
            animation: String::new(),
            extra: Extra::default(),
        }
    }
}

/// Both representations are returned. The game chooses 2D, 3D, or both.
pub struct Presentation<'a> {
    pub character: &'a Character,
    pub image: Option<&'a str>,
    pub model: Option<&'a ModelAsset>,
}
impl CharacterLibrary {
    pub fn presentation(&self, node: &StoryNode) -> Option<Presentation<'_>> {
        let character = self.characters.iter().find(|c| c.id == node.character)?;
        let image = if node.portrait.is_empty() {
            character.portraits.values().next()
        } else {
            character.portraits.get(&node.portrait)
        };
        let model = if node.model.is_empty() {
            character.models.values().next()
        } else {
            character.models.get(&node.model)
        };
        Some(Presentation {
            character,
            image: image.map(String::as_str),
            model,
        })
    }
}

impl Character {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    #[default]
    AtLeast,
    AtMost,
    Equal,
    NotEqual,
}
impl Comparison {
    pub fn label(self) -> &'static str {
        match self {
            Self::AtLeast => ">=",
            Self::AtMost => "<=",
            Self::Equal => "=",
            Self::NotEqual => "!=",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    Flag {
        flag: String,
        present: bool,
    },
    Attribute {
        key: String,
        comparison: Comparison,
        value: i64,
    },
}
impl Condition {
    pub fn label(&self) -> String {
        match self {
            Self::Flag { flag, present } => {
                format!("{}flag: {flag}", if *present { "Has " } else { "No " })
            }
            Self::Attribute {
                key,
                comparison,
                value,
            } => format!("{key} {} {value}", comparison.label()),
        }
    }
    pub fn met(&self, state: &PlayState) -> bool {
        match self {
            Self::Flag { flag, present } => state.flags.contains(flag) == *present,
            Self::Attribute {
                key,
                comparison,
                value,
            } => {
                // Missing attributes are not zero: typos must not silently unlock choices.
                let Some(actual) = state.attributes.get(key) else {
                    return false;
                };
                match comparison {
                    Comparison::AtLeast => actual >= value,
                    Comparison::AtMost => actual <= value,
                    Comparison::Equal => actual == value,
                    Comparison::NotEqual => actual != value,
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
    Flag { flag: String, present: bool },
    Set { key: String, value: i64 },
    Add { key: String, value: i64 },
    Event { name: String },
}
impl Effect {
    pub fn label(&self) -> String {
        match self {
            Self::Flag { flag, present } => {
                format!("{} {flag}", if *present { "Grant" } else { "Remove" })
            }
            Self::Set { key, value } => format!("{key} = {value}"),
            Self::Add { key, value } => format!("{key} {value:+}"),
            Self::Event { name } => format!("Event: {name}"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayState {
    pub node: String,
    pub flags: BTreeSet<String>,
    pub attributes: BTreeMap<String, i64>,
}
impl PlayState {
    pub fn start(
        nodes: &[StoryNode],
        library: &CharacterLibrary,
    ) -> Result<(Self, Vec<String>), String> {
        let first = nodes.first().ok_or("Add a story step first")?;
        let mut state = Self {
            node: first.id.clone(),
            ..Self::default()
        };
        for character in &library.characters {
            for (key, value) in &character.attributes {
                state
                    .attributes
                    .insert(format!("{}.{key}", character.id), *value);
            }
        }
        Ok((state, first.on_enter.clone()))
    }
    pub fn unmet(&self, choice: &Choice) -> Vec<String> {
        let mut unmet = Vec::new();
        if let Some(flag) = &choice.requires {
            if !self.flags.contains(flag) {
                unmet.push(format!("Requires flag: {flag}"));
            }
        }
        unmet.extend(
            choice
                .conditions
                .iter()
                .filter(|c| !c.met(self))
                .map(Condition::label),
        );
        unmet
    }
    /// Choice index is its position in the authored list, including hidden choices.
    /// Validate before applying; failed transitions leave the entire state unchanged.
    pub fn choose(&mut self, nodes: &[StoryNode], index: usize) -> Result<Vec<String>, String> {
        let node = nodes
            .iter()
            .find(|n| n.id == self.node)
            .ok_or("Current step is missing")?;
        let choice = node.choices.get(index).ok_or("Choice is missing")?;
        let unmet = self.unmet(choice);
        if !unmet.is_empty() {
            return Err(unmet.join("; "));
        }
        let target = nodes
            .iter()
            .find(|n| n.id == choice.target)
            .ok_or("Destination is missing")?;
        let mut next = self.clone();
        let mut events = Vec::new();
        if let Some(flag) = &choice.grants {
            next.flags.insert(flag.clone());
        }
        for effect in &choice.effects {
            match effect {
                Effect::Flag { flag, present } => {
                    if *present {
                        next.flags.insert(flag.clone());
                    } else {
                        next.flags.remove(flag);
                    }
                }
                Effect::Set { key, value } => {
                    next.attributes.insert(key.clone(), *value);
                }
                Effect::Add { key, value } => {
                    let result = next
                        .attributes
                        .get(key)
                        .copied()
                        .unwrap_or(0)
                        .checked_add(*value)
                        .ok_or("Attribute overflow")?;
                    next.attributes.insert(key.clone(), result);
                }
                Effect::Event { name } => events.push(name.clone()),
            }
        }
        events.extend(target.on_enter.clone());
        next.node = target.id.clone();
        *self = next;
        Ok(events)
    }
}

#[derive(Clone, Debug)]
pub struct Issue {
    pub node: String,
    pub error: bool,
    pub message: String,
}

pub fn validate(nodes: &[StoryNode], library: &CharacterLibrary) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut add = |node: &str, error, message| {
        issues.push(Issue {
            node: node.into(),
            error,
            message,
        })
    };
    let mut ids = BTreeSet::new();
    let mut character_ids = BTreeSet::new();
    for c in &library.characters {
        if c.id.trim().is_empty() || !character_ids.insert(&c.id) {
            add(
                "",
                true,
                format!("Duplicate or empty character ID: {}", c.id),
            );
        }
        for model in c.models.values() {
            if !model.scale.is_finite() || model.scale <= 0.0 {
                add("", true, format!("Invalid model scale for {}", c.name));
            }
        }
    }
    if nodes.is_empty() {
        add("", true, "Story has no steps".into());
    }
    let granted: BTreeSet<&str> = nodes
        .iter()
        .flat_map(|n| &n.choices)
        .flat_map(|c| {
            c.grants
                .iter()
                .map(String::as_str)
                .chain(c.effects.iter().filter_map(|e| match e {
                    Effect::Flag {
                        flag,
                        present: true,
                    } => Some(flag.as_str()),
                    _ => None,
                }))
        })
        .collect();
    for n in nodes {
        if n.id.trim().is_empty() || !ids.insert(&n.id) {
            add(&n.id, true, format!("Duplicate or empty step ID: {}", n.id));
        }
        if !n.character.is_empty() {
            match library.characters.iter().find(|c| c.id == n.character) {
                None => add(&n.id, true, format!("Unknown character: {}", n.character)),
                Some(c) => {
                    if !n.portrait.is_empty() && !c.portraits.contains_key(&n.portrait) {
                        add(&n.id, true, format!("Unknown portrait: {}", n.portrait));
                    }
                    if !n.model.is_empty() && !c.models.contains_key(&n.model) {
                        add(&n.id, true, format!("Unknown model: {}", n.model));
                    }
                }
            }
        } else if !n.portrait.is_empty() || !n.model.is_empty() {
            add(&n.id, true, "Presentation needs a character".into());
        }
        for c in &n.choices {
            if !nodes.iter().any(|n| n.id == c.target) {
                add(&n.id, true, format!("Missing destination: {}", c.target));
            }
            for flag in c
                .requires
                .iter()
                .chain(c.conditions.iter().filter_map(|c| match c {
                    Condition::Flag {
                        flag,
                        present: true,
                    } => Some(flag),
                    _ => None,
                }))
            {
                if !granted.contains(flag.as_str()) {
                    add(
                        &n.id,
                        false,
                        format!("Flag '{flag}' is never granted here (may come from the game)"),
                    );
                }
            }
        }
    }
    // Structural reachability only: conditions depend on the path and external game state.
    let mut reachable = BTreeSet::new();
    let mut pending: Vec<&str> = nodes
        .first()
        .map(|n| vec![n.id.as_str()])
        .unwrap_or_default();
    while let Some(id) = pending.pop() {
        if reachable.insert(id) {
            if let Some(n) = nodes.iter().find(|n| n.id == id) {
                pending.extend(n.choices.iter().map(|c| c.target.as_str()));
            }
        }
    }
    for n in nodes {
        if !reachable.contains(n.id.as_str()) {
            add(
                &n.id,
                false,
                "No connection from entry (ignoring conditions)".into(),
            );
        }
    }
    issues
}

#[cfg(test)]
mod tests;
