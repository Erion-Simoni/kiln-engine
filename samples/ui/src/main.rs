//! UI system demo.
//!
//! Demonstrates the kiln-ui API for creating widgets, layout, focus navigation,
//! and localization. This is a console demo since full rendering integration
//! is stubbed.

use kiln_ui::{
    Button, FocusDirection, FocusManager, Label, Layout, List, Localization, LocalizationKey,
    Panel, Widget, WidgetId,
};

fn main() {
    println!("=== Kiln UI System Demo ===\n");

    // Demonstrate localization
    println!("1. Localization");
    let mut loc = Localization::new("en");
    loc.add("menu.start", "Start Game");
    loc.add("menu.settings", "Settings");
    loc.add("menu.quit", "Quit");
    loc.add("level.name", "Forest Temple");

    println!("   English translations:");
    println!(
        "   - menu.start: {}",
        loc.get_or_key(&LocalizationKey::new("menu.start"))
    );
    println!(
        "   - menu.settings: {}",
        loc.get_or_key(&LocalizationKey::new("menu.settings"))
    );
    println!(
        "   - level.name: {}",
        loc.get_or_key(&LocalizationKey::new("level.name"))
    );
    println!(
        "   - missing.key: {}",
        loc.get_or_key(&LocalizationKey::new("missing.key"))
    );
    println!();

    // Demonstrate widgets
    println!("2. Widget Creation");
    let panel_id = WidgetId::new();
    let button_start_id = WidgetId::new();
    let button_settings_id = WidgetId::new();
    let button_quit_id = WidgetId::new();
    let label_title_id = WidgetId::new();
    let list_levels_id = WidgetId::new();

    let panel =
        Panel::new(panel_id).with_layout(Layout::new().with_anchor(kiln_ui::Anchor::Center));

    let button_start = Button::new(button_start_id, "menu.start")
        .with_layout(Layout::new().with_anchor(kiln_ui::Anchor::TopCenter));

    let _button_settings = Button::new(button_settings_id, "menu.settings")
        .with_layout(Layout::new().with_anchor(kiln_ui::Anchor::Center));

    let _button_quit = Button::new(button_quit_id, "menu.quit")
        .with_layout(Layout::new().with_anchor(kiln_ui::Anchor::BottomCenter));

    let label_title = Label::new(label_title_id, "level.name")
        .with_layout(Layout::new().with_anchor(kiln_ui::Anchor::TopCenter));

    let mut list_levels = List::new(list_levels_id)
        .with_layout(Layout::new().with_anchor(kiln_ui::Anchor::CenterLeft))
        .with_items(vec![
            LocalizationKey::new("level1"),
            LocalizationKey::new("level2"),
            LocalizationKey::new("level3"),
        ]);

    println!("   Created panel, 3 buttons, 1 label, 1 list");
    println!("   Panel ID: {:?}", panel.id());
    println!(
        "   Button start text key: {}",
        button_start.text_key().as_str()
    );
    println!("   Label text key: {}", label_title.text_key().as_str());
    println!("   List has {} items", list_levels.items().len());
    println!();

    // Demonstrate focus navigation
    println!("3. Focus Navigation");
    let mut focus_manager = FocusManager::new();

    focus_manager.register(button_start_id);
    focus_manager.register(button_settings_id);
    focus_manager.register(button_quit_id);
    focus_manager.register(list_levels_id);

    focus_manager.set_navigation(button_start_id, FocusDirection::Down, button_settings_id);
    focus_manager.set_navigation(button_settings_id, FocusDirection::Down, button_quit_id);
    focus_manager.set_navigation(button_settings_id, FocusDirection::Up, button_start_id);
    focus_manager.set_navigation(button_quit_id, FocusDirection::Up, button_settings_id);

    focus_manager.set_focused(Some(button_start_id));

    println!("   Registered {} focusable widgets", focus_manager.len());
    println!("   Initial focus: {:?}", focus_manager.focused());

    let next = focus_manager.move_focus(FocusDirection::Down);
    println!("   After Down: {:?}", next);

    let next = focus_manager.move_focus(FocusDirection::Down);
    println!("   After Down: {:?}", next);

    let next = focus_manager.move_focus(FocusDirection::Up);
    println!("   After Up: {:?}", next);

    let next = focus_manager.move_focus(FocusDirection::Next);
    println!("   After Next (linear): {:?}", next);
    println!();

    // Demonstrate list selection
    println!("4. List Selection");
    println!("   Selected index: {:?}", list_levels.selected_index());
    list_levels.set_selected_index(Some(1));
    println!(
        "   After set_selected_index(1): {:?}",
        list_levels.selected_index()
    );
    println!();

    // Demonstrate layout constraints
    println!("5. Layout Constraints");
    let constraints = kiln_ui::LayoutConstraints::new()
        .with_min_width(100.0)
        .with_max_width(800.0)
        .with_min_height(50.0)
        .with_max_height(600.0)
        .with_preferred_width(400.0)
        .with_preferred_height(300.0);

    let size = kiln_ui::Size::new(500.0, 200.0);
    let clamped = constraints.clamp_size(size);
    println!("   Original size: {}x{}", size.width, size.height);
    println!("   Clamped size: {}x{}", clamped.width, clamped.height);
    println!();

    println!("=== Demo Complete ===");
    println!();
    println!("Note: Full rendering integration with kiln-render is stubbed.");
    println!("The UI system provides:");
    println!("  - Widget types: Panel, Button, Label, List");
    println!("  - Layout system: anchors, flex, constraints");
    println!("  - Focus navigation: directional and linear");
    println!("  - Localization: key-based string lookup");
    println!("  - 9-slice sprites: scalable UI elements");
    println!("  - Text rendering: SDF/MSDF support (deferred)");
}
