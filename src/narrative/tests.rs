use super::*;

fn legacy() -> Vec<StoryNode> {
    serde_json::from_str(
        r#"[
      {"id":"opening","speaker":"Bera","text":"Hello","custom":{"voice":"a.ogg"},"choices":[
        {"text":"Take flute","target":"gate","grants":"flute"}]},
      {"id":"gate","speaker":"Bera","text":"Gate","choices":[
        {"text":"Play","target":"end","requires":"flute"}]},
      {"id":"end","speaker":"","text":"End","choices":[]}
    ]"#,
    )
    .unwrap()
}

#[test]
fn legacy_roundtrip_preserves_unknown_fields_and_flag_behavior() {
    let nodes = legacy();
    let json = serde_json::to_value(&nodes).unwrap();
    assert_eq!(json[0]["custom"]["voice"], "a.ogg");
    let mut state = PlayState::start(&nodes, &CharacterLibrary::default())
        .unwrap()
        .0;
    state.choose(&nodes, 0).unwrap();
    assert!(state.flags.contains("flute"));
    state.choose(&nodes, 0).unwrap();
    assert_eq!(state.node, "end");
}

#[test]
fn locked_or_broken_choices_do_not_mutate_state() {
    let mut nodes = legacy();
    let mut state = PlayState::start(&nodes, &CharacterLibrary::default())
        .unwrap()
        .0;
    state.node = "gate".into();
    let before = state.clone();
    assert!(state.choose(&nodes, 0).is_err());
    assert_eq!(state, before);
    state.flags.insert("flute".into());
    nodes[1].choices[0].target = "missing".into();
    let before = state.clone();
    assert!(state.choose(&nodes, 0).is_err());
    assert_eq!(state, before);
}

#[test]
fn character_attributes_conditions_and_events_share_runtime_semantics() {
    let mut nodes = legacy();
    let mut library = CharacterLibrary::default();
    let mut character = Character::new("bera".into(), "Bera".into());
    character.attributes.insert("trust".into(), 2);
    library.characters.push(character);
    nodes[0].on_enter.push("scene:castle".into());
    nodes[0].choices[0].conditions.push(Condition::Attribute {
        key: "bera.trust".into(),
        comparison: Comparison::AtLeast,
        value: 2,
    });
    nodes[0].choices[0].effects.push(Effect::Add {
        key: "bera.trust".into(),
        value: 1,
    });
    nodes[0].choices[0].effects.push(Effect::Event {
        name: "audio:flute".into(),
    });
    nodes[1].on_enter.push("scene:gate".into());
    let (mut state, events) = PlayState::start(&nodes, &library).unwrap();
    assert_eq!(events, ["scene:castle"]);
    assert_eq!(
        state.choose(&nodes, 0).unwrap(),
        ["audio:flute", "scene:gate"]
    );
    assert_eq!(state.attributes["bera.trust"], 3);
}

#[test]
fn validation_reports_duplicates_targets_and_unreachable_steps() {
    let mut nodes = legacy();
    nodes[0].choices[0].target = "missing".into();
    nodes.push(nodes[0].clone());
    let issues = validate(&nodes, &CharacterLibrary::default());
    assert!(issues.iter().any(|i| i.message.contains("Duplicate")));
    assert!(issues.iter().any(|i| i.message.contains("missing")));
    assert!(issues.iter().any(|i| i.node == "gate" && !i.error));
}

#[test]
fn character_presentation_resolves_independent_2d_and_3d_variants() {
    let mut library = CharacterLibrary::default();
    let mut character = Character::new("hero".into(), "Hero".into());
    character
        .portraits
        .insert("neutral".into(), "portraits/hero.png".into());
    character.models.insert(
        "armored".into(),
        ModelAsset {
            path: "models/hero.glb".into(),
            scale: 1.5,
            animation: "Idle".into(),
            ..Default::default()
        },
    );
    library.characters.push(character);
    let node = StoryNode {
        character: "hero".into(),
        model: "armored".into(),
        ..Default::default()
    };
    let presentation = library.presentation(&node).unwrap();
    assert_eq!(presentation.image, Some("portraits/hero.png"));
    assert_eq!(presentation.model.unwrap().path, "models/hero.glb");
    assert_eq!(presentation.model.unwrap().scale, 1.5);
    let encoded = serde_json::to_string(&library).unwrap();
    let loaded: CharacterLibrary = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        loaded.presentation(&node).unwrap().model.unwrap().animation,
        "Idle"
    );
}

#[test]
fn models_do_not_require_a_portrait_and_missing_variants_are_reported() {
    let mut library = CharacterLibrary::default();
    let mut character = Character::new("hero".into(), "Hero".into());
    character.models.insert(
        "default".into(),
        ModelAsset {
            path: "models/hero.glb".into(),
            ..Default::default()
        },
    );
    library.characters.push(character);
    let mut node = StoryNode {
        id: "opening".into(),
        character: "hero".into(),
        ..Default::default()
    };
    assert!(library.presentation(&node).unwrap().image.is_none());
    assert!(library.presentation(&node).unwrap().model.is_some());
    node.model = "missing".into();
    assert!(validate(&[node], &library)
        .iter()
        .any(|i| i.error && i.message.contains("model")));
}

#[test]
fn unknown_nested_metadata_is_never_silently_discarded() {
    // Unknown condition/effect syntax must fail at load rather than vanish on save.
    assert!(serde_json::from_str::<Condition>(
        r#"{"kind":"flag","flag":"key","present":true,"custom":42}"#
    )
    .is_err());
    assert!(
        serde_json::from_str::<Effect>(r#"{"kind":"event","name":"bell","volume":0.5}"#).is_err()
    );
    let model: ModelAsset =
        serde_json::from_str(r#"{"path":"models/hero.glb","custom":{"material":"gold"}}"#).unwrap();
    assert_eq!(
        serde_json::to_value(model).unwrap()["custom"]["material"],
        "gold"
    );
}

#[test]
fn attribute_comparisons_require_defined_values_and_match_boundaries() {
    let mut state = PlayState::default();
    for comparison in [
        Comparison::AtLeast,
        Comparison::AtMost,
        Comparison::Equal,
        Comparison::NotEqual,
    ] {
        assert!(!Condition::Attribute {
            key: "trust".into(),
            comparison,
            value: 0
        }
        .met(&state));
    }
    state.attributes.insert("trust".into(), 3);
    for (comparison, threshold, expected) in [
        (Comparison::AtLeast, 3, true),
        (Comparison::AtLeast, 4, false),
        (Comparison::AtMost, 3, true),
        (Comparison::AtMost, 2, false),
        (Comparison::Equal, 3, true),
        (Comparison::Equal, 2, false),
        (Comparison::NotEqual, 3, false),
        (Comparison::NotEqual, 2, true),
    ] {
        assert_eq!(
            Condition::Attribute {
                key: "trust".into(),
                comparison,
                value: threshold
            }
            .met(&state),
            expected
        );
    }
}

#[test]
fn overflow_rolls_back_prior_effects_and_grants() {
    let mut nodes = legacy();
    nodes[0].choices[0].effects.push(Effect::Add {
        key: "coins".into(),
        value: 1,
    });
    let mut state = PlayState::start(&nodes, &Default::default()).unwrap().0;
    state.attributes.insert("coins".into(), i64::MAX);
    let before = state.clone();
    assert!(state.choose(&nodes, 0).is_err());
    assert_eq!(state, before);
}

#[test]
fn negative_flags_set_add_and_remove_effects_evaluate_in_order() {
    let mut nodes = legacy();
    nodes[0].choices[0].conditions.push(Condition::Flag {
        flag: "blocked".into(),
        present: false,
    });
    nodes[0].choices[0].effects = vec![
        Effect::Set {
            key: "coins".into(),
            value: 2,
        },
        Effect::Add {
            key: "coins".into(),
            value: -1,
        },
        Effect::Flag {
            flag: "flute".into(),
            present: false,
        },
    ];
    let mut state = PlayState::start(&nodes, &Default::default()).unwrap().0;
    state.flags.insert("blocked".into());
    assert!(state.choose(&nodes, 0).is_err());
    state.flags.remove("blocked");
    state.choose(&nodes, 0).unwrap();
    assert_eq!(state.attributes["coins"], 1);
    assert!(!state.flags.contains("flute"));
}

#[test]
fn steps_roundtrip_explicit_audio_and_legacy_steps_stay_silent() {
    let mut nodes = legacy();
    assert!(nodes[0].audio.is_empty());
    nodes[0].audio = "audio/dialogue/a different filename.ogg".into();
    let encoded = serde_json::to_value(&nodes).unwrap();
    assert_eq!(
        encoded[0]["audio"],
        "audio/dialogue/a different filename.ogg"
    );
    assert!(encoded[1].get("audio").is_none());
    let restored: Vec<StoryNode> = serde_json::from_value(encoded).unwrap();
    assert_eq!(restored[0].audio, nodes[0].audio);
}
