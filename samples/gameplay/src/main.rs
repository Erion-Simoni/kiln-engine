//! Gameplay content tools sample
//!
//! Demonstrates:
//! - Dialogue system with conditions and variables
//! - Quest system with state persistence
//! - Data tables with CSV import
//! - Inventory/stat system driven by data tables

use kiln_gameplay::{
    DataTable, DataValue, Dialogue, DialogueChoice, DialogueCondition, DialogueNode, ObjectiveType,
    Quest, QuestObjective, QuestState, VariableOperation, VariableSet,
};

fn main() {
    println!("=== Kiln Gameplay Content Tools Sample ===\n");

    // Dialogue system demo
    println!("--- Dialogue System ---");
    demo_dialogue();

    // Quest system demo
    println!("\n--- Quest System ---");
    demo_quests();

    // Data tables demo
    println!("\n--- Data Tables ---");
    demo_data_tables();

    // Inventory/stat system demo
    println!("\n--- Inventory/Stat System ---");
    demo_inventory_stats();
}

fn demo_dialogue() {
    let mut dialogue = Dialogue::new("npc_intro".to_string(), "start".to_string());

    // Create start node
    let start_node = DialogueNode {
        speaker_key: "villager".to_string(),
        text_key: "dialogue_villager_hello".to_string(),
        choices: vec![
            DialogueChoice {
                text_key: "dialogue_choice_greet".to_string(),
                target_node: "friendly".to_string(),
                condition: None,
                set_variables: vec![],
            },
            DialogueChoice {
                text_key: "dialogue_choice_insult".to_string(),
                target_node: "hostile".to_string(),
                condition: None,
                set_variables: vec![VariableSet {
                    variable: "friendship".to_string(),
                    operation: VariableOperation::Subtract(10),
                }],
            },
        ],
        condition: None,
        set_variables: vec![],
    };
    dialogue.add_node(start_node);

    // Create friendly response node
    let friendly_node = DialogueNode {
        speaker_key: "villager".to_string(),
        text_key: "dialogue_villager_friendly".to_string(),
        choices: vec![],
        condition: Some(DialogueCondition::VariableEquals {
            variable: "friendship".to_string(),
            value: 10,
        }),
        set_variables: vec![VariableSet {
            variable: "friendship".to_string(),
            operation: VariableOperation::Add(5),
        }],
    };
    dialogue.add_node(friendly_node);

    println!("Dialogue ID: {}", dialogue.id);
    println!("Start node: {}", dialogue.start_node);
    println!("Total nodes: {}", dialogue.nodes.len());

    // Test condition evaluation
    let mut variables = std::collections::HashMap::new();
    variables.insert("friendship".to_string(), 10);

    let condition = DialogueCondition::VariableEquals {
        variable: "friendship".to_string(),
        value: 10,
    };
    println!(
        "Condition 'friendship == 10': {}",
        dialogue.check_condition(
            &Some(condition),
            &variables,
            &std::collections::HashMap::new()
        )
    );
}

fn demo_quests() {
    let mut quest = Quest::new(
        "main_quest".to_string(),
        "quest_main_name".to_string(),
        "quest_main_desc".to_string(),
    );

    // Add objectives
    quest.add_objective(QuestObjective {
        id: "obj1".to_string(),
        description_key: "quest_obj_kill_goblins".to_string(),
        target_type: ObjectiveType::KillEnemies {
            enemy_id: "goblin".to_string(),
        },
        target_value: 5,
        is_optional: false,
    });

    quest.add_objective(QuestObjective {
        id: "obj2".to_string(),
        description_key: "quest_obj_collect_coins".to_string(),
        target_type: ObjectiveType::CollectItems {
            item_id: "coin".to_string(),
        },
        target_value: 10,
        is_optional: false,
    });

    println!("Quest ID: {}", quest.id);
    println!("Objectives: {}", quest.objectives.len());

    // Create quest state
    let mut state = QuestState::new(&quest);
    state.start(1000);

    println!("Quest status: {:?}", state.status);

    // Update objective progress
    state.update_objective("obj1", 3);
    println!(
        "Objective 1 progress: {}",
        state.get_objective_progress("obj1").unwrap()
    );

    state.update_objective("obj1", 5);
    state.update_objective("obj2", 10);
    state.complete(2000);

    println!("Quest status after completion: {:?}", state.status);
    println!(
        "All objectives complete: {}",
        state.all_objectives_complete(&quest)
    );
}

fn demo_data_tables() {
    let csv_data = "id,name,damage,type,weight\nString,String,Integer,String,Integer\nitem1,Sword,10,Weapon,3\nitem2,Axe,15,Weapon,5\nitem3,Shield,0,Armor,8\nitem4,Potion,0,Consumable,1";

    let mut table = DataTable::new("items".to_string());
    table.import_from_csv(csv_data).unwrap();

    println!("Table ID: {}", table.id);
    println!("Columns: {}", table.columns.len());
    println!("Rows: {}", table.rows.len());

    // Get a specific row
    if let Some(row) = table.get_row("item1") {
        println!("\nItem 1:");
        println!("  Name: {:?}", row.values.get("name"));
        println!("  Damage: {:?}", row.values.get("damage"));
        println!("  Type: {:?}", row.values.get("type"));
    }

    // Find all weapons
    let weapons = table.find_rows("type", &DataValue::String("Weapon".to_string()));
    println!("\nWeapons found: {}", weapons.len());

    // Export to CSV
    let exported = table.export_to_csv();
    println!("\nExported CSV (first 100 chars):");
    println!("{}", &exported[..exported.len().min(100)]);
}

fn demo_inventory_stats() {
    // Create items data table
    let csv_data = "id,name,attack,defense,weight,value\nString,String,Integer,Integer,Integer,Integer\nsword1,Iron Sword,10,5,3,100\nsword2,Steel Sword,15,3,4,200\narmor1,Leather Armor,0,10,5,150\narmor2,Chainmail,2,20,10,300\npotion1,Health Potion,0,0,1,50";

    let mut items_table = DataTable::new("items".to_string());
    items_table.import_from_csv(csv_data).unwrap();

    println!("Items table loaded: {} items", items_table.rows.len());

    // Create player stats
    let mut player_stats = std::collections::HashMap::new();
    player_stats.insert("level".to_string(), 5i64);
    player_stats.insert("strength".to_string(), 10i64);
    player_stats.insert("gold".to_string(), 500i64);

    println!("\nPlayer stats:");
    for (stat, value) in &player_stats {
        println!("  {}: {}", stat, value);
    }

    // Simulate inventory: player has sword1 and armor1
    let inventory = vec!["sword1".to_string(), "armor1".to_string()];

    println!("\nPlayer inventory:");
    let mut total_attack = 0;
    let mut total_defense = 0;
    let mut total_weight = 0;

    for item_id in &inventory {
        if let Some(row) = items_table.get_row(item_id) {
            let name = row
                .values
                .get("name")
                .and_then(|v| v.as_string())
                .unwrap_or("Unknown");
            let attack = row
                .values
                .get("attack")
                .and_then(|v| v.as_integer())
                .unwrap_or(0);
            let defense = row
                .values
                .get("defense")
                .and_then(|v| v.as_integer())
                .unwrap_or(0);
            let weight = row
                .values
                .get("weight")
                .and_then(|v| v.as_integer())
                .unwrap_or(0);

            println!(
                "  - {} (ATK: {}, DEF: {}, WT: {})",
                name, attack, defense, weight
            );

            total_attack += attack;
            total_defense += defense;
            total_weight += weight;
        }
    }

    println!("\nDerived stats from inventory:");
    println!("  Total Attack: {}", total_attack);
    println!("  Total Defense: {}", total_defense);
    println!("  Total Weight: {}", total_weight);

    // Add player strength to attack
    let base_strength = player_stats.get("strength").copied().unwrap_or(0);
    let effective_attack = total_attack + base_strength;
    println!("  Effective Attack (with STR): {}", effective_attack);

    // Check if player can afford potion
    if let Some(potion_row) = items_table.get_row("potion1") {
        let potion_value = potion_row
            .values
            .get("value")
            .and_then(|v| v.as_integer())
            .unwrap_or(0);
        let player_gold = player_stats.get("gold").copied().unwrap_or(0);

        println!("\nHealth Potion costs {} gold", potion_value);
        println!("Player has {} gold", player_gold);
        println!("Can afford: {}", player_gold >= potion_value);
    }
}
