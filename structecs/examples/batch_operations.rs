#![allow(clippy::print_stdout, clippy::unwrap_used)]

//! # Batch Operations Example
//!
//! This example demonstrates batch operations in structecs for efficient bulk operations:
//! - `add_entities()` - Add multiple entities at once
//! - `try_remove_entities()` - Remove with error tracking
//! - `remove_entities()` - Remove without error tracking (fastest)
//!
//! ## When to Use Batch Operations
//!
//! **Use `add_entities()` when:**
//! - You need to add many entities of the same type
//! - Performance is critical (reduces atomic operations)
//! - You're initializing a game level or scene
//!
//! **Use `try_remove_entities()` when:**
//! - You need to know which entities failed to remove
//! - Error handling is important
//! - You're removing entities from mixed sources
//!
//! **Use `remove_entities()` when:**
//! - Maximum performance is needed
//! - You don't care about individual failures
//! - You're clearing large groups of entities (e.g., despawning effects)

use std::time::Instant;
use structecs::*;

#[derive(Debug, Extractable)]
pub struct Entity {
    pub name: String,
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Player {
    pub entity: Entity,
    pub health: u32,
    pub level: u32,
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Monster {
    pub entity: Entity,
    pub damage: u32,
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Item {
    pub entity: Entity,
    pub value: u32,
}

fn main() {
    println!("=== Batch Operations Example ===\n");

    let world = World::default();

    // ========================================
    // Example 1: Batch Addition
    // ========================================
    println!("--- Example 1: Batch Entity Addition ---\n");

    // Create a batch of players
    let players: Vec<Player> = (0..5)
        .map(|i| Player {
            entity: Entity {
                name: format!("Player{}", i),
            },
            health: 100,
            level: i + 1,
        })
        .collect();

    println!("Adding {} players in batch...", players.len());
    let start = Instant::now();
    let player_ids = world.add_entities(players);
    let batch_add_time = start.elapsed();
    println!("✓ Batch add: {:?}", batch_add_time);
    println!("  Entity IDs: {:?}\n", player_ids);

    // Compare with individual additions
    println!("Adding 5 monsters individually...");
    let start = Instant::now();
    let mut monster_ids = Vec::new();
    for i in 0..5 {
        let id = world.add_entity(Monster {
            entity: Entity {
                name: format!("Monster{}", i),
            },
            damage: 10 + i * 5,
        });
        monster_ids.push(id);
    }
    let individual_add_time = start.elapsed();
    println!("✓ Individual add: {:?}", individual_add_time);
    println!(
        "  Speedup: {:.2}x faster with batch\n",
        individual_add_time.as_nanos() as f64 / batch_add_time.as_nanos() as f64
    );

    println!("Total entities: {}", world.entity_count());
    println!("Total archetypes: {}\n", world.archetype_count());

    // ========================================
    // Example 2: Batch Removal with Error Tracking
    // ========================================
    println!("--- Example 2: Batch Removal with Error Tracking ---\n");

    // Remove first 3 players
    let to_remove = &player_ids[0..3];
    println!("Removing {} players...", to_remove.len());

    match world.try_remove_entities(to_remove) {
        Ok(()) => println!("✓ Successfully removed all {} entities", to_remove.len()),
        Err(e) => println!("✗ Error: {:?}", e),
    }

    println!("Remaining entities: {}\n", world.entity_count());

    // Try to remove with invalid IDs
    println!("Attempting to remove mix of valid and invalid IDs...");
    let mixed_ids = vec![
        player_ids[3],            // Valid
        EntityId::from_raw(9999), // Invalid
        player_ids[4],            // Valid
        EntityId::from_raw(8888), // Invalid
    ];

    match world.try_remove_entities(&mixed_ids) {
        Ok(()) => println!("✓ All entities removed successfully"),
        Err(WorldError::PartialRemoval { succeeded, failed }) => {
            println!("⚠ Partial removal:");
            println!("  Succeeded: {} entities {:?}", succeeded.len(), succeeded);
            println!("  Failed: {} entities {:?}", failed.len(), failed);
        }
        Err(e) => println!("✗ Error: {:?}", e),
    }

    println!("Remaining entities: {}\n", world.entity_count());

    // ========================================
    // Example 3: Fast Batch Removal (No Error Tracking)
    // ========================================
    println!("--- Example 3: Fast Batch Removal (No Error Tracking) ---\n");

    // Add many items
    let items: Vec<Item> = (0..100)
        .map(|i| Item {
            entity: Entity {
                name: format!("Item{}", i),
            },
            value: i * 10,
        })
        .collect();

    let item_ids = world.add_entities(items);
    println!("Added {} items", item_ids.len());
    println!("Total entities: {}\n", world.entity_count());

    // Fast removal with some invalid IDs mixed in
    let mut ids_to_remove = item_ids.clone();
    ids_to_remove.push(EntityId::from_raw(7777)); // Invalid ID
    ids_to_remove.push(EntityId::from_raw(6666)); // Invalid ID

    println!("Fast removing {} IDs (including 2 invalid)...", ids_to_remove.len());
    let start = Instant::now();
    world.remove_entities(&ids_to_remove);
    let fast_remove_time = start.elapsed();
    println!("✓ Fast remove: {:?}", fast_remove_time);
    println!("  (Invalid IDs silently skipped)");
    println!("Remaining entities: {}\n", world.entity_count());

    // Compare with error tracking version
    let items2: Vec<Item> = (100..200)
        .map(|i| Item {
            entity: Entity {
                name: format!("Item{}", i),
            },
            value: i * 10,
        })
        .collect();
    let item_ids2 = world.add_entities(items2);

    println!("Comparing removal methods with {} entities...", item_ids2.len());

    let mut ids_with_invalid = item_ids2.clone();
    ids_with_invalid.push(EntityId::from_raw(5555));
    ids_with_invalid.push(EntityId::from_raw(4444));

    // Benchmark try_remove_entities
    let start = Instant::now();
    let _ = world.try_remove_entities(&ids_with_invalid);
    let tracked_remove_time = start.elapsed();
    println!("  try_remove_entities(): {:?}", tracked_remove_time);

    // Re-add for fair comparison
    let items3: Vec<Item> = (200..300)
        .map(|i| Item {
            entity: Entity {
                name: format!("Item{}", i),
            },
            value: i * 10,
        })
        .collect();
    let item_ids3 = world.add_entities(items3);
    let mut ids_with_invalid2 = item_ids3.clone();
    ids_with_invalid2.push(EntityId::from_raw(3333));
    ids_with_invalid2.push(EntityId::from_raw(2222));

    // Benchmark remove_entities
    let start = Instant::now();
    world.remove_entities(&ids_with_invalid2);
    let untracked_remove_time = start.elapsed();
    println!("  remove_entities():     {:?}", untracked_remove_time);

    println!(
        "  Speedup: {:.2}x faster without error tracking\n",
        tracked_remove_time.as_nanos() as f64 / untracked_remove_time.as_nanos() as f64
    );

    // ========================================
    // Example 4: Large-Scale Batch Operations
    // ========================================
    println!("--- Example 4: Large-Scale Performance Test ---\n");

    // Test with 10,000 entities
    let count = 10_000;
    println!("Creating {} monsters...", count);

    let large_batch: Vec<Monster> = (0..count)
        .map(|i| Monster {
            entity: Entity {
                name: format!("Monster{}", i),
            },
            damage: i % 50,
        })
        .collect();

    let start = Instant::now();
    let large_batch_ids = world.add_entities(large_batch);
    let large_batch_add = start.elapsed();
    println!("✓ Batch add (10k): {:?}", large_batch_add);
    println!("  Avg per entity: {:?}", large_batch_add / count);

    // Compare with individual adds
    println!("\nComparing with individual adds (1000 entities)...");
    let start = Instant::now();
    for i in 0..1000 {
        world.add_entity(Monster {
            entity: Entity {
                name: format!("MonsterIndividual{}", i),
            },
            damage: i % 50,
        });
    }
    let individual_add_1k = start.elapsed();
    println!("Individual add (1k): {:?}", individual_add_1k);
    println!("  Avg per entity: {:?}", individual_add_1k / 1000);

    let batch_per_entity = large_batch_add.as_nanos() / count as u128;
    let individual_per_entity = individual_add_1k.as_nanos() / 1000;
    println!(
        "\n✓ Batch is {:.2}x faster per entity",
        individual_per_entity as f64 / batch_per_entity as f64
    );

    // Large batch removal
    println!("\nRemoving all {} monsters in batch...", large_batch_ids.len());
    let start = Instant::now();
    world.remove_entities(&large_batch_ids);
    let large_batch_remove = start.elapsed();
    println!("✓ Batch remove (10k): {:?}", large_batch_remove);
    println!("  Avg per entity: {:?}", large_batch_remove / count);

    println!("\nFinal entity count: {}", world.entity_count());

    // ========================================
    // Summary
    // ========================================
    println!("\n=== Summary ===\n");
    println!("✓ Batch operations demonstration complete!");
    println!("\nKey Takeaways:");
    println!("  • add_entities() is significantly faster than repeated add_entity()");
    println!("  • try_remove_entities() provides error tracking for mixed ID sets");
    println!("  • remove_entities() is fastest when you don't need error tracking");
    println!("  • Batch operations reduce atomic operations and archetype lookups");
    println!("\nPerformance Tips:");
    println!("  • Use batch operations for level initialization");
    println!("  • Use remove_entities() for despawning large groups (e.g., particles)");
    println!("  • Use try_remove_entities() when entity existence is uncertain");
    println!("  • Group entities by type when possible for better cache locality");
}
