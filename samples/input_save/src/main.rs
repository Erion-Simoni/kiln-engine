//! Sample demonstrating input mapping and save system.

use kiln_input::{Action, ActionMap, Binding, BindingType};
use kiln_save::{SaveSlotManager, SaveVersion, VersionMigration, MigrationError};
use kiln_runtime::{GraphicsSettings, Resolution};
use chrono;

fn main() {
    println!("=== Kiln Input Mapping and Save System Demo ===\n");

    demo_action_maps();
    demo_graphics_settings();
    demo_save_system();
    demo_save_migration();
}

fn demo_action_maps() {
    println!("--- Action Maps ---");

    let mut action_map = ActionMap::new();

    // Create actions
    let jump_action = Action::new("jump", "Jump")
        .with_binding(Binding::new(BindingType::Key("Space".to_string())));

    let fire_action = Action::new("fire", "Fire")
        .with_binding(Binding::new(BindingType::Key("KeyZ".to_string())));

    action_map.add_action(jump_action);
    action_map.add_action(fire_action);

    println!("Added actions: jump, fire");

    // Serialize
    let json = action_map.to_json().unwrap();
    println!("Serialized action map:\n{}", json);

    // Deserialize
    let mut deserialized = ActionMap::from_json(&json).unwrap();
    println!("Deserialized {} actions", deserialized.actions.len());

    // Rebind jump action
    if let Some(action) = deserialized.get_action_mut("jump") {
        action.bindings.clear();
        action.add_binding(Binding::new(BindingType::Key("KeyJ".to_string())));
        println!("Rebound jump to 'J' key");
    }

    println!();
}

fn demo_graphics_settings() {
    println!("--- Graphics Settings ---");

    let settings = GraphicsSettings::new()
        .with_resolution(Resolution::HD)
        .with_vsync(false)
        .with_frame_cap(Some(60));

    println!("Resolution: {}", settings.resolution);
    println!("VSync: {}", settings.vsync);
    println!("Frame cap: {:?}", settings.frame_cap);

    let json = settings.to_json().unwrap();
    println!("Serialized settings:\n{}", json);

    let deserialized = GraphicsSettings::from_json(&json).unwrap();
    println!("Deserialized resolution: {}", deserialized.resolution);

    println!();
}

fn demo_save_system() {
    println!("--- Save System ---");

    let temp_dir = std::env::temp_dir().join("kiln_input_save_demo");
    let _ = std::fs::create_dir_all(&temp_dir);

    let manager = SaveSlotManager::with_directory(temp_dir.clone());

    // Save game data
    let game_data = r#"{"player": {"x": 100, "y": 200, "score": 1000}}"#;
    manager.save(0, game_data, "Chapter 1 completed").unwrap();
    println!("Saved game to slot 0");

    // List slots
    let slots = manager.list_slots().unwrap();
    println!("Found {} save slot(s)", slots.len());
    for slot in &slots {
        println!("  Slot {}: {} ({})", slot.slot_id, slot.description, slot.version);
    }

    // Load game data
    let loaded = manager.load(0).unwrap();
    println!("Loaded data: {}", loaded.data);

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);

    println!();
}

struct V1ToV2Migration;

impl VersionMigration for V1ToV2Migration {
    fn from_version(&self) -> SaveVersion {
        SaveVersion::new(1, 0, 0)
    }

    fn to_version(&self) -> SaveVersion {
        SaveVersion::new(1, 1, 0)
    }

    fn migrate(&self, data: &str) -> Result<String, MigrationError> {
        // Simulate migration: add new field
        let without_brace = data.trim_end_matches('}');
        let migrated = format!("{}, \"version\": \"v2\"}}", without_brace);
        Ok(migrated)
    }
}

fn demo_save_migration() {
    println!("--- Save Migration ---");

    let temp_dir = std::env::temp_dir().join("kiln_save_migration_demo");
    let _ = std::fs::create_dir_all(&temp_dir);

    let mut manager = SaveSlotManager::with_directory(temp_dir.clone());
    manager.register_migration(Box::new(V1ToV2Migration));

    // Create a v1 save manually
    let v1_data = r#"{"player": {"x": 100, "y": 200}}"#;
    let metadata = kiln_save::SaveSlotMetadata {
        slot_id: 0,
        version: SaveVersion::new(1, 0, 0),
        timestamp: chrono::Utc::now().timestamp(),
        description: "V1 save".to_string(),
    };
    let slot = kiln_save::SaveSlot {
        metadata,
        data: v1_data.to_string(),
    };
    let slot_path = temp_dir.join("slot_0.json");
    let json = serde_json::to_string_pretty(&slot).unwrap();
    std::fs::write(&slot_path, json).unwrap();
    println!("Created v1 save file");

    // Load (should trigger migration to current version 1.1.0)
    let loaded = manager.load(0).unwrap();
    println!("Loaded and migrated data: {}", loaded.data);
    println!("New version: {}", loaded.metadata.version);

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);

    println!();
}
