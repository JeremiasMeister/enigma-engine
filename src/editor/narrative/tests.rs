use super::*;
#[test]
fn undo_redo_restores_character_and_story_edits() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = NarrativeEditor {
        document: Some(Document::create(dir.path(), "test").unwrap()),
        ..Default::default()
    };
    let before = editor.snapshot().unwrap();
    editor.document.as_mut().unwrap().nodes[0].text = "Changed".into();
    editor
        .document
        .as_mut()
        .unwrap()
        .library
        .characters
        .push(crate::narrative::Character::new(
            "hero".into(),
            "Hero".into(),
        ));
    editor.record(before);
    editor.undo();
    assert!(editor
        .document
        .as_ref()
        .unwrap()
        .library
        .characters
        .is_empty());
    assert_eq!(
        editor.document.as_ref().unwrap().nodes[0].text,
        "Your story begins here."
    );
    editor.redo();
    assert_eq!(editor.document.as_ref().unwrap().nodes[0].text, "Changed");
    assert_eq!(
        editor.document.as_ref().unwrap().library.characters.len(),
        1
    );
}
#[test]
fn all_workspace_modes_render_with_legacy_and_character_content() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = NarrativeEditor::default();
    let mut doc = Document::create(dir.path(), "test").unwrap();
    doc.library
        .characters
        .push(crate::narrative::Character::new(
            "hero".into(),
            "Hero".into(),
        ));
    editor.document = Some(doc);
    let ctx = egui::Context::default();
    for mode in [
        Mode::Story,
        Mode::Graph,
        Mode::Characters,
        Mode::Dependencies,
        Mode::Preview,
    ] {
        editor.mode = mode;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1080.0, 720.0),
                )),
                ..Default::default()
            },
            |ctx| editor.draw(ctx, Some(dir.path())),
        );
        assert!(!output.shapes.is_empty());
    }
}

#[test]
fn inspecting_characters_preserves_per_step_speaker_overrides() {
    let dir = tempfile::tempdir().unwrap();
    let mut editor = NarrativeEditor::default();
    let mut doc = Document::create(dir.path(), "test").unwrap();
    doc.library
        .characters
        .push(Character::new("hero".into(), "Hero".into()));
    doc.nodes[0].character = "hero".into();
    doc.nodes[0].speaker = "Mysterious stranger".into();
    let ctx = Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| characters::draw(ui, &mut editor, &mut doc));
    });
    assert_eq!(doc.nodes[0].speaker, "Mysterious stranger");
}

#[test]
fn failed_load_and_project_switch_keep_the_current_draft() {
    let original = tempfile::tempdir().unwrap();
    let next = tempfile::tempdir().unwrap();
    let mut editor = NarrativeEditor {
        document: Some(Document::create(original.path(), "draft").unwrap()),
        ..Default::default()
    };
    editor.document.as_mut().unwrap().nodes[0].text = "Keep me".into();
    editor.perform(Pending::Load(
        next.path().into(),
        next.path().join("src/resources/story/missing.json"),
    ));
    assert_eq!(editor.document.as_ref().unwrap().nodes[0].text, "Keep me");
    let ctx = Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        editor.draw(ctx, Some(next.path()))
    });
    assert_eq!(editor.document.as_ref().unwrap().root, original.path());
    assert_eq!(editor.document.as_ref().unwrap().nodes[0].text, "Keep me");
}

#[test]
fn narrative_imports_register_project_assets_without_duplicates() {
    let root = tempfile::tempdir().unwrap();
    let mut project = crate::editor::state::ProjectState::new(
        "test".into(),
        root.path().to_string_lossy().into_owned(),
    );
    let mut editor = NarrativeEditor {
        imports: vec![
            (root.path().into(), "models/hero.glb".into(), true),
            (root.path().into(), "portraits/hero.png".into(), false),
            (root.path().into(), "models/hero.glb".into(), true),
        ],
        ..Default::default()
    };
    assert!(editor.register_imports(&mut project));
    assert_eq!(project.manifest.len(), 2);
    assert_eq!(
        project.manifest[0].kind,
        crate::editor::state::ResourceKind::Model
    );
    assert_eq!(
        project.manifest[1].kind,
        crate::editor::state::ResourceKind::Texture
    );
    assert!(!editor.register_imports(&mut project));
}

#[test]
fn build_saves_valid_drafts_and_rejects_invalid_or_conflicted_stories() {
    let root = tempfile::tempdir().unwrap();
    let mut editor = NarrativeEditor {
        document: Some(Document::create(root.path(), "chapter").unwrap()),
        ..Default::default()
    };
    editor.save_project(root.path()).unwrap();
    assert!(!editor.dirty());
    editor.document.as_mut().unwrap().nodes[0]
        .choices
        .push(crate::narrative::Choice {
            text: "Broken".into(),
            target: "missing".into(),
            ..Default::default()
        });
    assert!(editor.save_project(root.path()).is_err());
    editor.document.as_mut().unwrap().nodes[0].choices.clear();
    editor.document.as_mut().unwrap().nodes[0].text = "Draft".into();
    std::fs::write(&editor.document.as_ref().unwrap().path, "External edit").unwrap();
    assert!(editor.save_project(root.path()).is_err());
    assert!(editor.dirty());
}

#[test]
fn continuations_and_branches_create_reachable_unique_destinations() {
    let mut nodes = vec![StoryNode {
        id: "step_1".into(),
        text: "Start".into(),
        ..Default::default()
    }];
    let next = add_connected_step(&mut nodes, 0, "Continue").unwrap();
    assert_eq!(nodes[0].choices[0].target, nodes[next].id);
    let branch = add_connected_step(&mut nodes, 0, "Take the forest path").unwrap();
    assert_ne!(nodes[next].id, nodes[branch].id);
    assert_eq!(nodes[0].choices.len(), 2);
    assert_eq!(nodes[0].choices[1].text, "Take the forest path");
    let library = CharacterLibrary::default();
    assert!(crate::narrative::validate(&nodes, &library).is_empty());
    let (mut play, _) = PlayState::start(&nodes, &library).unwrap();
    play.choose(&nodes, 1).unwrap();
    assert_eq!(play.node, nodes[branch].id);
    let original = nodes.clone();
    assert_eq!(add_connected_step(&mut nodes, 999, "Invalid"), None);
    assert_eq!(nodes, original);
}

#[test]
fn connected_step_undo_redo_restores_both_destination_and_link() {
    let root = tempfile::tempdir().unwrap();
    let mut editor = NarrativeEditor {
        document: Some(Document::create(root.path(), "chapter").unwrap()),
        ..Default::default()
    };
    let before = editor.snapshot().unwrap();
    add_connected_step(&mut editor.document.as_mut().unwrap().nodes, 0, "Continue");
    let after = editor.snapshot().unwrap();
    editor.record(before.clone());
    editor.undo();
    assert!(editor.snapshot().unwrap() == before);
    editor.redo();
    assert!(editor.snapshot().unwrap() == after);
}

#[test]
fn writing_is_default_and_navigation_uses_dialogue() {
    assert!(NarrativeEditor::default().mode == Mode::Story);
    let node = StoryNode {
        id: "opaque_42".into(),
        speaker: "Bera".into(),
        text: "Welcome home.".into(),
        ..Default::default()
    };
    assert_eq!(step_label(&node), "Bera: Welcome home.");
    assert_eq!(step_label(&StoryNode::default()), "Unwritten moment");
}

#[test]
fn selecting_step_audio_saves_resource_relative_audio_data() {
    let root = tempfile::tempdir().unwrap();
    let audio_dir = root.path().join("src/resources/audio");
    std::fs::create_dir_all(&audio_dir).unwrap();
    std::fs::write(audio_dir.join("greeting.mp3"), []).unwrap();
    let mut doc = Document::create(root.path(), "audio_story").unwrap();
    let mut editor = NarrativeEditor::default();
    editor.reload_assets(root.path());
    let ctx = Context::default();
    let mut render = |events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 1600.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .show(ctx, |ui| inspector::story(ui, &mut editor, &mut doc));
            },
        )
    };
    let mut output = render(vec![]);
    for label in ["None", "audio/greeting.mp3"] {
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    Some(text.pos + egui::vec2(5.0, 5.0))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Audio option not rendered: {label}"));
        for pressed in [true, false] {
            render(vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
        }
        output = render(vec![]);
    }
    assert_eq!(doc.nodes[0].audio, "audio/greeting.mp3");
    doc.save().unwrap();
    let json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&doc.path).unwrap()).unwrap();
    assert_eq!(json[0]["audio"], "audio/greeting.mp3");
    assert_eq!(
        Document::load(root.path(), &doc.path).unwrap().nodes[0].audio,
        "audio/greeting.mp3"
    );
}
