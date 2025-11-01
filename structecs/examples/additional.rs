//! Example demonstrating the Additional components feature.
//!
//! Additional components allow you to attach optional, dynamic data to entities
//! without modifying the base entity structure. This is useful for:
//! - Temporary states (buffs, debuffs, status effects)
//! - Optional metadata (tags, markers, flags)
//! - Runtime-determined data (quest progress, achievements)
//!
//! Run this example with:
//! ```bash
//! cargo run --example additional
//! ```

use structecs::{Extractable, World};

// Base entity structure
#[derive(Debug, Extractable)]
struct Player {
    name: String,
    health: u32,
    level: u32,
}

// Optional additional components
#[derive(Debug, Extractable)]
struct Buff {
    name: String,
    power: u32,
    duration_seconds: u32,
}

#[derive(Debug, Extractable)]
struct Poisoned {
    damage_per_tick: u32,
    ticks_remaining: u32,
}

#[derive(Debug, Extractable)]
struct QuestProgress {
    quest_id: String,
    objectives_completed: u32,
    total_objectives: u32,
}

fn main() {
    println!("=== structecs Additional Components Example ===\n");

    let world = World::new();

    // Create base players
    println!("1. Creating players...");
    let warrior_id = world.add_entity(Player {
        name: "Warrior".to_string(),
        health: 150,
        level: 10,
    });

    let mage_id = world.add_entity(Player {
        name: "Mage".to_string(),
        health: 80,
        level: 12,
    });

    let rogue_id = world.add_entity(Player {
        name: "Rogue".to_string(),
        health: 100,
        level: 8,
    });

    println!("   Created {} players\n", 3);

    // Add buffs to some players
    println!("2. Applying buffs...");
    world.add_additional(
        &warrior_id,
        Buff {
            name: "Strength".to_string(),
            power: 50,
            duration_seconds: 30,
        },
    );

    world.add_additional(
        &mage_id,
        Buff {
            name: "Intelligence".to_string(),
            power: 75,
            duration_seconds: 60,
        },
    );

    println!("   Warrior: +50 Strength for 30s");
    println!("   Mage: +75 Intelligence for 60s\n");

    // Apply poison to rogue
    println!("3. Applying poison...");
    world.add_additional(
        &rogue_id,
        Poisoned {
            damage_per_tick: 5,
            ticks_remaining: 10,
        },
    );
    println!("   Rogue is poisoned! (5 damage/tick, 10 ticks)\n");

    // Add quest progress to warrior
    println!("4. Adding quest progress...");
    world.add_additional(
        &warrior_id,
        QuestProgress {
            quest_id: "main_quest_001".to_string(),
            objectives_completed: 3,
            total_objectives: 5,
        },
    );
    println!("   Warrior: Quest 'main_quest_001' (3/5 objectives)\n");

    // Query all players and their additionals
    println!("5. Querying players with buffs and poison status...");
    for (id, player, (buff, poison)) in world.query_with::<Player, (Buff, Poisoned)>().query() {
        print!(
            "   [{}] {} (HP: {}, Lv: {})",
            id.id(),
            player.name,
            player.health,
            player.level
        );

        if let Some(b) = buff {
            print!(" | Buff: {} (+{})", b.name, b.power);
        }

        if let Some(p) = poison {
            print!(
                " | POISONED (-{} x {})",
                p.damage_per_tick, p.ticks_remaining
            );
        }

        println!();
    }
    println!();

    // Check specific additional
    println!("6. Checking quest progress...");
    if world.has_additional::<QuestProgress>(&warrior_id)
        && let Some(quest) = world.extract_additional::<QuestProgress>(&warrior_id)
    {
        println!(
            "   Warrior's quest '{}': {}/{} objectives completed",
            quest.quest_id, quest.objectives_completed, quest.total_objectives
        );
    }
    println!();

    // Replace buff with stronger one
    println!("7. Replacing warrior's buff with stronger one...");
    world.add_additional(
        &warrior_id,
        Buff {
            name: "Super Strength".to_string(),
            power: 100,
            duration_seconds: 15,
        },
    );

    if let Some(buff) = world.extract_additional::<Buff>(&warrior_id) {
        println!(
            "   Warrior now has: {} (+{} for {}s)",
            buff.name, buff.power, buff.duration_seconds
        );
    }
    println!();

    // Remove poison
    println!("8. Curing rogue's poison...");
    if let Some(poison) = world.remove_additional::<Poisoned>(&rogue_id) {
        println!(
            "   Removed poison: {} damage/tick with {} ticks remaining",
            poison.damage_per_tick, poison.ticks_remaining
        );
    }
    println!();

    // Final status
    println!("9. Final player status:");
    for (id, player) in world.query::<Player>() {
        println!("   {} (HP: {})", player.name, player.health);

        if world.has_additional::<Buff>(&id) {
            let buff = world.extract_additional::<Buff>(&id).unwrap();
            println!("      - Buff: {} (+{})", buff.name, buff.power);
        }

        if world.has_additional::<Poisoned>(&id) {
            println!("      - STATUS: Poisoned");
        }

        if world.has_additional::<QuestProgress>(&id) {
            let quest = world.extract_additional::<QuestProgress>(&id).unwrap();
            println!(
                "      - Quest: {} ({}/{})",
                quest.quest_id, quest.objectives_completed, quest.total_objectives
            );
        }
    }

    println!("\n=== Example Complete ===");
}
