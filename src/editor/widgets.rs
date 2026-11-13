use std::ops::RangeInclusive;

use egui::{Color32, DragValue, Response, RichText, Slider, Ui};

use super::theme;

/// The range is a useful dragging range, not a restriction on authored data.
/// Keep the value editable and never clamp existing values just by displaying them.
pub fn slider(ui: &mut Ui, label: &str, value: &mut f32, range: RangeInclusive<f32>) -> bool {
    slider_with(ui, label, value, range, "", false).changed()
}

pub fn slider_with(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    suffix: &str,
    logarithmic: bool,
) -> Response {
    ui.push_id(label, |ui| {
        ui.label(label);
        ui.scope(|ui| {
            ui.spacing_mut().slider_width = (ui.available_width() - 82.0).max(40.0);
            ui.add(
                Slider::new(value, range)
                    .clamp_to_range(false)
                    .logarithmic(logarithmic)
                    .suffix(suffix),
            )
            .on_hover_text("Drag to adjust. Click the number to enter an exact value.")
        })
        .inner
    })
    .inner
}

/// Show radians as degrees, writing back only after an actual interaction.
pub fn angle_slider(
    ui: &mut Ui,
    label: &str,
    radians: &mut f32,
    degrees: RangeInclusive<f32>,
) -> bool {
    let mut value = radians.to_degrees();
    let changed = slider_with(ui, label, &mut value, degrees, "°", false).changed();
    if changed {
        *radians = value.to_radians();
    }
    changed
}

pub fn vector3(ui: &mut Ui, label: &str, value: &mut [f32; 3], speed: f64) -> bool {
    ui.push_id(label, |ui| {
        ui.label(label);
        let mut changed = false;
        ui.horizontal(|ui| {
            let width = ((ui.available_width() - 16.0) / 3.0).max(44.0);
            for (index, (axis, color)) in [
                ("X", Color32::from_rgb(239, 139, 144)),
                ("Y", Color32::from_rgb(145, 202, 160)),
                ("Z", theme::ACCENT),
            ]
            .into_iter()
            .enumerate()
            {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    ui.label(RichText::new(axis).small().strong().color(color));
                    changed |= ui
                        .add_sized(
                            [width - 14.0, 25.0],
                            DragValue::new(&mut value[index]).speed(speed),
                        )
                        .changed();
                });
            }
        });
        changed
    })
    .inner
}

pub fn matches_search(label: &str, query: &str) -> bool {
    let label = label.to_lowercase();
    query
        .split_whitespace()
        .all(|word| label.contains(&word.to_lowercase()))
}

pub fn search_field(ui: &mut Ui, query: &mut String, hint: &str) -> Response {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 25.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            // Measure the clear button first, then give the field the remaining
            // width. A guessed button width overflows and grows resizable panels
            // (and picker popups) again on every frame.
            let clear = ui
                .add_enabled(!query.is_empty(), egui::Button::new("×"))
                .on_hover_text("Clear search")
                .clicked();
            if clear {
                query.clear();
            }
            let response = ui.add_sized(
                [ui.available_width(), 25.0],
                egui::TextEdit::singleline(query).hint_text(hint),
            );
            if clear {
                response.request_focus();
            }
            response
        },
    )
    .inner
}

/// Each picker keeps its own query. Searching never changes the selected value.
pub fn searchable_combo<T: Clone + PartialEq>(
    ui: &mut Ui,
    id_source: impl std::hash::Hash,
    selected: &mut T,
    options: &[(T, String)],
) -> bool {
    searchable_combo_with_label(ui, id_source, selected, options, "Missing reference")
}

fn searchable_combo_with_label<T: Clone + PartialEq>(
    ui: &mut Ui,
    id_source: impl std::hash::Hash,
    selected: &mut T,
    options: &[(T, String)],
    fallback: &str,
) -> bool {
    let id = ui.make_persistent_id(id_source);
    let search_id = id.with("search");
    let open_id = id.with("was_open");
    let was_open = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(open_id).unwrap_or(false));
    let mut query = ui
        .ctx()
        .data(|data| data.get_temp::<String>(search_id).unwrap_or_default());
    let current = options
        .iter()
        .find(|(value, _)| value == selected)
        .map(|(_, name)| name.as_str())
        .unwrap_or(fallback);
    let mut changed = false;
    let mut picked = false;
    let popup_id = id.with("popup");
    let button = ui.add(egui::Button::new(current).min_size(egui::vec2(120.0, 24.0)));
    if button.clicked() {
        ui.memory_mut(|memory| memory.toggle_popup(popup_id));
    }
    if ui.memory(|memory| memory.is_popup_open(popup_id)) {
        let popup = egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .constrain(true)
            .fixed_pos(button.rect.left_bottom())
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(260.0);
                    if !was_open {
                        query.clear();
                    }
                    let search = search_field(ui, &mut query, "Search options…");
                    if !was_open {
                        search.request_focus();
                    }
                    ui.separator();
                    let matches: Vec<_> = options
                        .iter()
                        .filter(|(_, label)| matches_search(label, &query))
                        .collect();
                    let enter = (search.has_focus() || search.lost_focus())
                        && ui.input(|input| input.key_pressed(egui::Key::Enter));
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .show(ui, |ui| {
                            if matches.is_empty() {
                                ui.weak("No matching options");
                            }
                            for (index, (value, label)) in matches.iter().enumerate() {
                                // Distinct IDs keep duplicate asset names independently selectable.
                                let clicked = ui
                                    .push_id(index, |ui| {
                                        ui.selectable_label(*value == *selected, label.as_str())
                                            .clicked()
                                    })
                                    .inner;
                                if clicked || (enter && index == 0) {
                                    picked = true;
                                    if *selected != *value {
                                        *selected = (*value).clone();
                                        changed = true;
                                    }
                                }
                            }
                        });
                    ui.small(format!("{} of {} options", matches.len(), options.len()));
                });
            });
        if picked
            || ui.input(|input| input.key_pressed(egui::Key::Escape))
            || (button.clicked_elsewhere() && popup.response.clicked_elsewhere())
        {
            ui.memory_mut(|memory| memory.close_popup());
        }
    }
    let is_open = ui.memory(|memory| memory.is_popup_open(popup_id));
    ui.ctx().data_mut(|data| {
        data.insert_temp(search_id, query);
        data.insert_temp(open_id, is_open);
    });
    changed
}

/// A searchable one-shot choice list, sharing popup and keyboard behavior with value pickers.
pub fn searchable_action<T: Clone + PartialEq>(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    label: &str,
    options: &[(T, String)],
) -> Option<T> {
    let mut selected = None;
    let choices: Vec<_> = options
        .iter()
        .map(|(value, label)| (Some(value.clone()), label.clone()))
        .collect();
    searchable_combo_with_label(ui, id, &mut selected, &choices, label);
    selected
}

pub fn asset_picker(
    ui: &mut Ui,
    label: &str,
    selected: &mut Option<uuid::Uuid>,
    assets: &[(uuid::Uuid, String)],
    empty_label: &str,
) -> bool {
    ui.push_id(label, |ui| {
        ui.label(label);
        let mut options = vec![(None, empty_label.to_owned())];
        options.extend(assets.iter().map(|(id, name)| (Some(*id), name.clone())));
        searchable_combo(ui, "asset", selected, &options)
    })
    .inner
}

pub fn enum_picker(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    current: &'static str,
    choices: &[&'static str],
) -> Option<&'static str> {
    let mut options: Vec<_> = choices
        .iter()
        .map(|label| (*label, (*label).to_owned()))
        .collect();
    if !choices.contains(&current) {
        options.push((current, current.to_owned()));
    }
    let mut selected = current;
    searchable_combo(ui, id, &mut selected, &options).then_some(selected)
}

/// Searchable action lists for scene switching and spawning assets.
pub fn search_menu<T: Clone>(ui: &mut Ui, label: &str, options: &[(T, String)]) -> Option<T> {
    let id = ui.make_persistent_id(("search_menu", label));
    let open_id = id.with("open");
    let was_open = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(open_id).unwrap_or(false));
    let mut query = ui
        .ctx()
        .data(|data| data.get_temp::<String>(id).unwrap_or_default());
    let mut selected = None;
    let response = ui.menu_button(label, |ui| {
        ui.set_width(240.0);
        if !was_open {
            query.clear();
        }
        let search = search_field(ui, &mut query, "Search options…");
        if !was_open {
            search.request_focus();
        }
        ui.separator();
        let enter = (search.has_focus() || search.lost_focus())
            && ui.input(|input| input.key_pressed(egui::Key::Enter));
        let matches: Vec<_> = options
            .iter()
            .filter(|(_, name)| matches_search(name, &query))
            .collect();
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                if matches.is_empty() {
                    ui.weak("No matching options");
                }
                for (index, (value, name)) in matches.iter().enumerate() {
                    if ui.button(name.as_str()).clicked() || (enter && index == 0) {
                        selected = Some((*value).clone());
                        ui.close_menu();
                    }
                }
            });
        ui.small(format!("{} of {} options", matches.len(), options.len()));
    });
    ui.ctx().data_mut(|data| {
        data.insert_temp(id, query);
        data.insert_temp(open_id, response.inner.is_some() && selected.is_none());
    });
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_field_does_not_grow_its_sidebar_between_frames() {
        let mut query = String::new();
        let ctx = egui::Context::default();
        let mut first_width = None;
        for _ in 0..20 {
            ctx.begin_frame(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1080.0, 720.0),
                )),
                ..Default::default()
            });
            theme::apply(&ctx);
            let panel = egui::SidePanel::right("inspector")
                .default_width(310.0)
                .min_width(280.0)
                .frame(theme::panel_frame())
                .resizable(true)
                .show(&ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            theme::panel_heading(ui, "Inspector", "Scene settings");
                            search_field(ui, &mut query, "Search…");
                        });
                });
            let width = panel.response.rect.width();
            let _ = ctx.end_frame();
            let initial = *first_width.get_or_insert(width);
            assert!(
                (width - initial).abs() < 0.5,
                "idle inspector grew from {initial} to {width}"
            );
        }
    }

    fn frame(ctx: &egui::Context, draw: impl FnOnce(&mut Ui)) -> egui::FullOutput {
        frame_with_input(ctx, Vec::new(), draw)
    }

    fn frame_with_input(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        draw: impl FnOnce(&mut Ui),
    ) -> egui::FullOutput {
        ctx.begin_frame(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(280.0, 300.0),
            )),
            events,
            ..Default::default()
        });
        theme::apply(ctx);
        egui::CentralPanel::default().show(ctx, draw);
        ctx.end_frame()
    }

    fn click(ctx: &egui::Context, pos: egui::Pos2, mut draw: impl FnMut(&mut Ui)) {
        for pressed in [true, false] {
            frame_with_input(
                ctx,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                &mut draw,
            );
        }
    }

    fn text_present(output: &egui::FullOutput, expected: &str) -> bool {
        fn matches(shape: &egui::epaint::Shape, expected: &str) -> bool {
            match shape {
                egui::epaint::Shape::Text(text) => text.galley.job.text == expected,
                egui::epaint::Shape::Vec(shapes) => {
                    shapes.iter().any(|shape| matches(shape, expected))
                }
                _ => false,
            }
        }
        output
            .shapes
            .iter()
            .any(|shape| matches(&shape.shape, expected))
    }

    #[test]
    fn typing_a_precise_slider_value_does_not_round_it_to_zero() {
        let ctx = egui::Context::default();
        let mut value = 0.05;
        let mut rect = egui::Rect::NOTHING;
        frame(&ctx, |ui| {
            rect = slider_with(ui, "Noise scale", &mut value, 0.001..=0.5, "", false).rect
        });
        click(
            &ctx,
            egui::pos2(rect.right() - 10.0, rect.center().y),
            |ui| {
                slider(ui, "Noise scale", &mut value, 0.001..=0.5);
            },
        );
        frame_with_input(
            &ctx,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                },
                egui::Event::Text("0.0001".into()),
            ],
            |ui| {
                slider(ui, "Noise scale", &mut value, 0.001..=0.5);
            },
        );
        assert!((value - 0.0001).abs() < 1e-8, "typed value was {value}");
    }

    #[test]
    fn picker_filters_without_changing_selection_then_enter_selects_a_match() {
        let ctx = egui::Context::default();
        let options = vec![
            (0, "None".into()),
            (1, "Rock slate".into()),
            (2, "Grass".into()),
        ];
        let mut selected = 0;
        frame(&ctx, |ui| {
            searchable_combo(ui, "picker", &mut selected, &options);
        });
        click(&ctx, egui::pos2(60.0, 20.0), |ui| {
            searchable_combo(ui, "picker", &mut selected, &options);
        });
        let output = frame_with_input(&ctx, vec![egui::Event::Text("SLATE rock".into())], |ui| {
            assert!(!searchable_combo(ui, "picker", &mut selected, &options));
        });
        assert_eq!(selected, 0);
        assert!(text_present(&output, "Rock slate"));
        assert!(!text_present(&output, "Grass"));
        frame_with_input(
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            |ui| {
                searchable_combo(ui, "picker", &mut selected, &options);
            },
        );
        assert_eq!(selected, 1);
        let closed = frame(&ctx, |ui| {
            searchable_combo(ui, "picker", &mut selected, &options);
        });
        assert!(
            !text_present(&closed, "1 of 3 options"),
            "Enter must close the popup"
        );
    }

    #[test]
    fn clicking_a_picker_search_field_keeps_the_popup_open() {
        let ctx = egui::Context::default();
        let options = vec![(0, "None".into()), (1, "Rock".into())];
        let mut selected = 0;
        frame(&ctx, |ui| {
            searchable_combo(ui, "picker", &mut selected, &options);
        });
        click(&ctx, egui::pos2(60.0, 20.0), |ui| {
            searchable_combo(ui, "picker", &mut selected, &options);
        });
        click(&ctx, egui::pos2(60.0, 55.0), |ui| {
            searchable_combo(ui, "picker", &mut selected, &options);
        });
        let output = frame(&ctx, |ui| {
            searchable_combo(ui, "picker", &mut selected, &options);
        });
        assert!(
            text_present(&output, "2 of 2 options"),
            "clicking inside search must keep the popup open"
        );
    }

    #[test]
    fn picker_search_focus_dismissal_and_queries_are_independent() {
        let ctx = egui::Context::default();
        let options = vec![(0, "None".into()), (1, "Rock".into()), (2, "Grass".into())];
        let mut selected = 0;
        let mut draw = |ui: &mut Ui| {
            searchable_combo(ui, "first", &mut selected, &options);
        };
        frame(&ctx, &mut draw);
        click(&ctx, egui::pos2(60.0, 20.0), &mut draw);
        frame(&ctx, &mut draw);
        click(&ctx, egui::pos2(60.0, 55.0), &mut draw);
        let output = frame_with_input(&ctx, vec![egui::Event::Text("Rock".into())], &mut draw);
        assert!(text_present(&output, "1 of 3 options"));
        assert!(ctx.memory(|m| m.any_popup_open()));
        frame_with_input(
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut draw,
        );
        assert!(!ctx.memory(|m| m.any_popup_open()));
        assert_eq!(selected, 0);
        let mut draw_second = |ui: &mut Ui| {
            searchable_combo(ui, "second", &mut selected, &options);
        };
        frame(&ctx, &mut draw_second);
        click(&ctx, egui::pos2(60.0, 20.0), &mut draw_second);
        let output = frame(&ctx, &mut draw_second);
        assert!(
            text_present(&output, "3 of 3 options"),
            "another picker must not inherit a query"
        );
        click(&ctx, egui::pos2(270.0, 290.0), &mut draw_second);
        assert!(!ctx.memory(|m| m.any_popup_open()));
        assert_eq!(selected, 0);
    }

    #[test]
    fn picker_with_no_matches_preserves_its_reference() {
        let ctx = egui::Context::default();
        let options = vec![(7, "Rock".into())];
        let mut selected = 7;
        frame(&ctx, |ui| {
            searchable_combo(ui, "picker", &mut selected, &options);
        });
        click(&ctx, egui::pos2(60.0, 20.0), |ui| {
            searchable_combo(ui, "picker", &mut selected, &options);
        });
        let output = frame_with_input(&ctx, vec![egui::Event::Text("unknown".into())], |ui| {
            assert!(!searchable_combo(ui, "picker", &mut selected, &options));
        });
        assert!(text_present(&output, "No matching options"));
        assert_eq!(selected, 7);
    }

    #[test]
    fn opening_slider_preserves_values_outside_its_dragging_range() {
        for original in [-0.5_f32, 3.75] {
            let ctx = egui::Context::default();
            let mut value = original;
            for _ in 0..2 {
                frame(&ctx, |ui| {
                    assert!(!slider(ui, "Strength", &mut value, 0.0..=1.0));
                });
                assert_eq!(value.to_bits(), original.to_bits());
            }
        }
    }

    #[test]
    fn camera_angle_displays_degrees_without_rewriting_radians() {
        let ctx = egui::Context::default();
        let mut fov = std::f32::consts::FRAC_PI_2;
        let original = fov.to_bits();
        let output = frame(&ctx, |ui| {
            assert!(!angle_slider(ui, "Field of view", &mut fov, 5.0..=150.0));
        });
        assert_eq!(fov.to_bits(), original);
        fn has_degrees(shape: &egui::epaint::Shape) -> bool {
            match shape {
                egui::epaint::Shape::Text(text) => {
                    let label = &text.galley.job.text;
                    label.contains("90") && label.contains('°')
                }
                egui::epaint::Shape::Vec(shapes) => shapes.iter().any(has_degrees),
                _ => false,
            }
        }
        assert!(output.shapes.iter().any(|shape| has_degrees(&shape.shape)));
    }

    #[test]
    fn controls_fit_inside_a_narrow_inspector() {
        let ctx = egui::Context::default();
        frame(&ctx, |ui| {
            let right = ui.max_rect().right();
            slider(ui, "Roughness", &mut 0.5, 0.0..=1.0);
            vector3(ui, "Position", &mut [1.0, 2.0, 3.0], 0.05);
            assert!(
                ui.min_rect().right() <= right + 1.0,
                "property controls overflow their panel"
            );
        });
    }
}
