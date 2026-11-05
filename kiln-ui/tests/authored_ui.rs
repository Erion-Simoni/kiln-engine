use kiln_core::ui::*;
use kiln_ui::{
    authored::{clip_quad, UiFonts, UiInput, UiRuntime, UiViewport},
    Rect, UiQuad,
};
use std::collections::BTreeMap;
fn button(id: &str, position: [f32; 2]) -> UiWidget {
    UiWidget {
        id: id.into(),
        layout: UiLayout {
            offset: position,
            size: [100.0, 36.0],
            ..Default::default()
        },
        ..Default::default()
    }
}
fn document() -> UiDocument {
    let mut a = button("a", [40.0, 40.0]);
    a.event = "open".into();
    a.action = UiAction::ShowScreen("settings".into());
    let b = button("b", [40.0, 100.0]);
    let mut c = button("c", [180.0, 100.0]);
    c.enabled = false;
    UiDocument {
        initial_screen: "title".into(),
        pause_screen: "title".into(),
        screens: BTreeMap::from([
            (
                "title".into(),
                UiScreen {
                    default_focus: "a".into(),
                    widgets: vec![a, b, c],
                    ..Default::default()
                },
            ),
            (
                "settings".into(),
                UiScreen {
                    widgets: vec![UiWidget {
                        id: "back".into(),
                        action: UiAction::Back,
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ),
        ]),
        ..Default::default()
    }
}
#[test]
fn navigation_edges_skip_disabled_widgets_and_do_not_retrigger_across_screens() {
    let mut ui = UiRuntime::new(&document(), "main").unwrap();
    assert_eq!(ui.focused(), Some("a"));
    let down = UiInput {
        down: true,
        ..Default::default()
    };
    ui.update(down.clone(), [640.0, 360.0]).unwrap();
    assert_eq!(ui.focused(), Some("b"));
    ui.update(down, [640.0, 360.0]).unwrap();
    assert_eq!(ui.focused(), Some("b"));
    ui.update(UiInput::default(), [640.0, 360.0]).unwrap();
    ui.update(
        UiInput {
            up: true,
            ..Default::default()
        },
        [640.0, 360.0],
    )
    .unwrap();
    assert_eq!(ui.focused(), Some("a"));
    let enter = UiInput {
        accept: true,
        ..Default::default()
    };
    let events = ui.update(enter.clone(), [640.0, 360.0]).unwrap();
    assert_eq!(events[0].name, "open");
    assert_eq!(events[0].screen, "title");
    assert_eq!(ui.active_screen(), "settings");
    assert_eq!(ui.focused(), Some("back"));
    assert!(ui.update(enter, [640.0, 360.0]).unwrap().is_empty());
    assert_eq!(ui.active_screen(), "settings");
    ui.update(UiInput::default(), [640.0, 360.0]).unwrap();
    ui.update(
        UiInput {
            cancel: true,
            ..Default::default()
        },
        [640.0, 360.0],
    )
    .unwrap();
    assert_eq!(ui.active_screen(), "title");
    ui.set_flag("a", false, false);
    assert_eq!(ui.focused(), Some("b"));
}
#[test]
fn hit_testing_uses_topmost_clipped_widgets_and_release_capture() {
    let mut doc = document();
    let s = doc.screens.get_mut("title").unwrap();
    s.widgets[0].parent = "panel".into();
    s.widgets[0].layout.offset = [80.0, 0.0];
    s.widgets.push(UiWidget {
        id: "panel".into(),
        kind: UiKind::Panel,
        layout: UiLayout {
            offset: [20.0, 20.0],
            size: [100.0, 60.0],
            ..Default::default()
        },
        ..Default::default()
    });
    let mut ui = UiRuntime::new(&doc, "").unwrap();
    let mouse = |point, down| UiInput {
        pointer: Some(point),
        pointer_down: down,
        ..Default::default()
    };
    ui.update(mouse([150.0, 30.0], true), [640.0, 360.0])
        .unwrap();
    assert!(ui
        .update(mouse([150.0, 30.0], false), [640.0, 360.0])
        .unwrap()
        .is_empty()); // Child is clipped after x=120.
    ui.update(mouse([110.0, 30.0], true), [640.0, 360.0])
        .unwrap();
    assert!(ui
        .update(mouse([140.0, 30.0], false), [640.0, 360.0])
        .unwrap()
        .is_empty()); // Released outside.
    ui.update(mouse([110.0, 30.0], true), [640.0, 360.0])
        .unwrap();
    ui.reset_input();
    assert!(ui
        .update(mouse([110.0, 30.0], false), [640.0, 360.0])
        .unwrap()
        .is_empty()); // Lost focus cancels capture.
    ui.update(mouse([110.0, 30.0], true), [640.0, 360.0])
        .unwrap();
    assert_eq!(
        ui.update(mouse([110.0, 30.0], false), [640.0, 360.0])
            .unwrap()[0]
            .widget,
        "a"
    );
}
#[test]
fn viewport_resize_anchors_and_textured_clipping_share_exact_geometry() {
    let view = UiViewport::new([100.0, 50.0, 1280.0, 720.0], [640.0, 360.0]);
    assert_eq!(view.logical_point([100.0, 50.0]), Some([0.0, 0.0]));
    assert_eq!(view.logical_point([99.0, 50.0]), None);
    let mut doc = document();
    let s = doc.screens.get_mut("title").unwrap();
    s.widgets.truncate(1);
    s.widgets[0].layout = UiLayout {
        anchor: [1.0, 1.0],
        pivot: [1.0, 1.0],
        offset: [-10.0, -10.0],
        size: [100.0, 36.0],
        ..Default::default()
    };
    let ui = UiRuntime::new(&doc, "").unwrap();
    let draw = ui.draw(&UiFonts::default(), view);
    assert_eq!(draw[0].instance.position, [1260.0, 714.0]);
    let wide = UiViewport::new([0.0, 0.0, 800.0, 360.0], [640.0, 360.0]);
    assert_eq!(
        ui.draw(&UiFonts::default(), wide)[0].instance.position,
        [740.0, 332.0]
    );
    let q = UiQuad::rect(0.0, 0.0, 100.0, 100.0, [1.0; 4]);
    let q = clip_quad(q, Rect::new(25.0, 10.0, 50.0, 40.0)).unwrap();
    assert_eq!(q.instance.position, [50.0, 30.0]);
    assert_eq!(q.instance.uv_rect, [0.25, 0.1, 0.75, 0.5]);
}
#[test]
fn invalid_references_geometry_cycles_and_navigation_fail_before_play() {
    let mut doc = document();
    doc.screens.get_mut("title").unwrap().widgets[1].id = "a".into();
    assert!(doc.validate().is_err());
    let mut doc = document();
    doc.screens.get_mut("title").unwrap().widgets[0].parent = "b".into();
    doc.screens.get_mut("title").unwrap().widgets[1].parent = "a".into();
    assert!(doc.validate().is_err());
    let mut doc = document();
    doc.screens.get_mut("title").unwrap().widgets[0]
        .layout
        .anchor[0] = f32::NAN;
    assert!(doc.validate().is_err());
    let mut doc = document();
    doc.screens.get_mut("title").unwrap().widgets[0]
        .navigation
        .insert("down".into(), "missing".into());
    assert!(doc.validate().is_err());
    let mut doc = document();
    doc.initial_screen = "missing".into();
    assert!(doc.validate().is_err());
}
#[test]
fn normalized_controller_tokens_have_the_same_navigation_and_accept_edges() {
    let mut ui = UiRuntime::new(&document(), "").unwrap();
    ui.update(UiInput::from_keys(["GamepadDown"]), [640.0, 360.0])
        .unwrap();
    assert_eq!(ui.focused(), Some("b"));
    ui.update(UiInput::default(), [640.0, 360.0]).unwrap();
    ui.update(UiInput::from_keys(["GamepadUp"]), [640.0, 360.0])
        .unwrap();
    assert_eq!(
        ui.update(UiInput::from_keys(["GamepadAccept"]), [640.0, 360.0])
            .unwrap()[0]
            .name,
        "open"
    );
}

#[cfg(feature = "fonts")]
#[test]
fn real_ttf_atlas_has_alpha_coverage_metrics_and_textured_glyph_uvs() {
    let bytes = include_bytes!("../../samples/ui2d_editor/assets/fonts/KilnSampleSans.ttf");
    let mut doc = document();
    doc.fonts.insert(
        "body".into(),
        UiFont {
            path: "font.ttf".into(),
            extra_chars: "Ëë".into(),
        },
    );
    let w = &mut doc.screens.get_mut("title").unwrap().widgets[0];
    w.text = "Ëngine".into();
    w.font = "body".into();
    let mut pixels = Vec::new();
    let (fonts, errors) = UiFonts::load(
        &doc,
        |_| Ok(bytes.to_vec()),
        |_, size, data| {
            assert_eq!(size, [1024, 1024]);
            pixels = data.to_vec();
            kiln_render::TextureId(9)
        },
    );
    assert!(errors.is_empty());
    assert!(pixels.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));
    let font = &fonts.fonts["body"];
    assert!(font.measure("WWW", 18.0)[0] > font.measure("iii", 18.0)[0]);
    let view = UiViewport::new([0.0, 0.0, 640.0, 360.0], doc.reference_size);
    let ui = UiRuntime::new(&doc, "").unwrap();
    let quads = ui.draw(&fonts, view);
    let glyphs: Vec<_> = quads
        .iter()
        .filter(|q| q.texture == kiln_render::TextureId(9))
        .collect();
    assert_eq!(glyphs.len(), 6);
    assert!(glyphs
        .iter()
        .all(|q| q.instance.uv_rect != [0.0, 0.0, 1.0, 1.0]));
    let fallback = font.render("☃", [0.0, 0.0], 18.0, [1.0; 4]);
    let question = font.render("?", [0.0, 0.0], 18.0, [1.0; 4]);
    assert_eq!(fallback[0].instance.uv_rect, question[0].instance.uv_rect);
}
#[test]
fn rapid_pointer_and_keyboard_taps_survive_a_single_host_tick() {
    let mut ui = UiRuntime::new(&document(), "").unwrap();
    let input = UiInput {
        pointer: Some([50.0, 50.0]),
        pointer_down: false,
        pointer_events: vec![(Some([50.0, 50.0]), true), (Some([50.0, 50.0]), false)],
        ..Default::default()
    };
    assert_eq!(ui.update(input, [640.0, 360.0]).unwrap()[0].name, "open");
    ui.update(
        UiInput {
            pressed_keys: vec!["Escape".into()],
            ..Default::default()
        },
        [640.0, 360.0],
    )
    .unwrap();
    assert_eq!(ui.active_screen(), "title");
    assert_eq!(
        ui.update(
            UiInput {
                pressed_keys: vec!["Enter".into()],
                ..Default::default()
            },
            [640.0, 360.0]
        )
        .unwrap()[0]
            .name,
        "open"
    );
    assert!(ui
        .update(UiInput::default(), [640.0, 360.0])
        .unwrap()
        .is_empty());
}
#[test]
fn oversized_bitmap_text_is_rejected_and_runtime_text_keeps_its_previous_value() {
    let mut doc = document();
    doc.screens.get_mut("title").unwrap().widgets[0].text = "W".repeat(1000);
    assert!(doc.validate().unwrap_err().contains("quad budget"));
    let mut ui = UiRuntime::new(&document(), "").unwrap();
    assert!(!ui.set_text("a", &"W".repeat(1000)));
    assert_eq!(ui.document().screens["title"].widgets[0].text, "Button");
}
