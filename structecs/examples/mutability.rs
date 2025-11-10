#![allow(clippy::print_stdout, clippy::unwrap_used)]

//! # Mutability Example
//!
//! This example demonstrates different approaches to mutable state in structecs:
//! - AtomicU32 for lock-free counter updates
//! - Mutex for exclusive access to complex state
//! - RwLock for multiple readers, single writer scenarios
//!
//! ## When to Use Each Type
//!
//! **AtomicU32/AtomicBool:**
//! - Best for simple counters, flags, or IDs
//! - Lock-free, highest performance
//! - Limited to primitive types and simple operations
//! - Use when: Simple numeric state, high contention expected
//!
//! **Mutex:**
//! - Exclusive access to complex data structures
//! - Simpler API than RwLock
//! - Blocks all readers and writers when locked
//! - Use when: Write operations are common, or reads are quick
//!
//! **RwLock:**
//! - Multiple readers OR one writer
//! - Better for read-heavy workloads
//! - More overhead than Mutex for write-heavy workloads
//! - Use when: Many readers, few writers

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use structecs::*;

// Example 1: Lock-free counter using AtomicU32
#[derive(Debug, Extractable)]
pub struct Entity {
    pub name: String,
    pub id: AtomicU32,
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Player {
    pub entity: Entity,
    // Lock-free counter for health updates
    pub health: AtomicU32,
    // Lock-free counter for score
    pub score: AtomicU32,
}

// Example 2: Complex state using Mutex
#[derive(Debug)]
pub struct Inventory {
    pub items: Vec<String>,
    pub capacity: usize,
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Merchant {
    pub entity: Entity,
    // Mutex for exclusive access to inventory
    pub inventory: Mutex<Inventory>,
}

// Example 3: Read-heavy state using RwLock
#[derive(Debug)]
pub struct GameStats {
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Warrior {
    pub entity: Entity,
    // RwLock for stats that are read often, written rarely
    pub stats: RwLock<GameStats>,
}

fn main() {
    println!("=== Mutability Example ===\n");

    let world = Arc::new(World::default());

    // ========================================
    // Example 1: AtomicU32 for lock-free updates
    // ========================================
    println!("--- Example 1: Lock-Free Atomic Operations ---\n");

    world.add_entity(Player {
        entity: Entity {
            name: "Hero".to_string(),
            id: AtomicU32::new(1),
        },
        health: AtomicU32::new(100),
        score: AtomicU32::new(0),
    });

    println!("Initial player state:");
    for (_id, player) in world.query::<Player>() {
        println!(
            "  {}: health={}, score={}",
            player.entity.name,
            player.health.load(Ordering::Relaxed),
            player.score.load(Ordering::Relaxed)
        );
    }

    // Simulate concurrent damage and score updates
    let mut handles = vec![];

    for i in 0..3 {
        let world = Arc::clone(&world);
        let handle = thread::spawn(move || {
            for (_id, player) in world.query::<Player>() {
                // Simulate taking damage (decrement)
                player.health.fetch_sub(5, Ordering::Relaxed);
                // Simulate gaining score (increment)
                player.score.fetch_add(10, Ordering::Relaxed);

                thread::sleep(Duration::from_millis(10));
            }
            println!("Thread {} completed damage/score updates", i + 1);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("\nAfter concurrent updates:");
    for (_id, player) in world.query::<Player>() {
        println!(
            "  {}: health={}, score={}",
            player.entity.name,
            player.health.load(Ordering::Relaxed),
            player.score.load(Ordering::Relaxed)
        );
    }

    // ========================================
    // Example 2: Mutex for exclusive access
    // ========================================
    println!("\n--- Example 2: Mutex for Complex State ---\n");

    world.add_entity(Merchant {
        entity: Entity {
            name: "Shopkeeper".to_string(),
            id: AtomicU32::new(2),
        },
        inventory: Mutex::new(Inventory {
            items: vec!["Sword".to_string(), "Shield".to_string()],
            capacity: 10,
        }),
    });

    println!("Initial merchant inventory:");
    for (_id, merchant) in world.query::<Merchant>() {
        let inv = merchant.inventory.lock().unwrap();
        println!("  {} has {} items: {:?}", merchant.entity.name, inv.items.len(), inv.items);
    }

    // Simulate concurrent inventory updates
    let mut handles = vec![];

    for i in 0..3 {
        let world = Arc::clone(&world);
        let handle = thread::spawn(move || {
            for (_id, merchant) in world.query::<Merchant>() {
                // Lock the inventory for writing
                let mut inv = merchant.inventory.lock().unwrap();
                if inv.items.len() < inv.capacity {
                    inv.items.push(format!("Item{}", i));
                }
                // Lock is automatically released when inv goes out of scope
            }
            println!("Thread {} added items to inventory", i + 1);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("\nAfter concurrent updates:");
    for (_id, merchant) in world.query::<Merchant>() {
        let inv = merchant.inventory.lock().unwrap();
        println!("  {} now has {} items: {:?}", merchant.entity.name, inv.items.len(), inv.items);
    }

    // ========================================
    // Example 3: RwLock for read-heavy workloads
    // ========================================
    println!("\n--- Example 3: RwLock for Read-Heavy Access ---\n");

    world.add_entity(Warrior {
        entity: Entity {
            name: "Gladiator".to_string(),
            id: AtomicU32::new(3),
        },
        stats: RwLock::new(GameStats {
            kills: 0,
            deaths: 0,
            assists: 0,
        }),
    });

    println!("Initial warrior stats:");
    for (_id, warrior) in world.query::<Warrior>() {
        let stats = warrior.stats.read().unwrap();
        println!(
            "  {}: K/D/A = {}/{}/{}",
            warrior.entity.name, stats.kills, stats.deaths, stats.assists
        );
    }

    // Simulate many readers and few writers
    let mut handles = vec![];

    // 5 reader threads
    for i in 0..5 {
        let world = Arc::clone(&world);
        let handle = thread::spawn(move || {
            for (_id, warrior) in world.query::<Warrior>() {
                // Multiple readers can acquire read locks simultaneously
                let stats = warrior.stats.read().unwrap();
                let kda = if stats.deaths == 0 {
                    stats.kills as f32
                } else {
                    (stats.kills as f32 + stats.assists as f32 / 2.0) / stats.deaths as f32
                };
                println!("Reader {}: KDA = {:.2}", i + 1, kda);
                thread::sleep(Duration::from_millis(5));
            }
        });
        handles.push(handle);
    }

    // 2 writer threads
    for i in 0..2 {
        let world = Arc::clone(&world);
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(10)); // Let readers start first
            for (_id, warrior) in world.query::<Warrior>() {
                // Only one writer can acquire write lock at a time
                let mut stats = warrior.stats.write().unwrap();
                stats.kills += 1;
                stats.assists += 2;
                println!("Writer {} updated stats", i + 1);
                thread::sleep(Duration::from_millis(5));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("\nFinal warrior stats:");
    for (_id, warrior) in world.query::<Warrior>() {
        let stats = warrior.stats.read().unwrap();
        println!(
            "  {}: K/D/A = {}/{}/{}",
            warrior.entity.name, stats.kills, stats.deaths, stats.assists
        );
    }

    // ========================================
    // Performance Comparison
    // ========================================
    println!("\n--- Performance Comparison ---\n");

    // Add 1000 test entities
    for i in 0..1000 {
        world.add_entity(Player {
            entity: Entity {
                name: format!("Player{}", i),
                id: AtomicU32::new(i),
            },
            health: AtomicU32::new(100),
            score: AtomicU32::new(0),
        });
    }

    // Benchmark: Atomic operations
    let start = std::time::Instant::now();
    for (_id, player) in world.query::<Player>() {
        player.score.fetch_add(1, Ordering::Relaxed);
    }
    let atomic_time = start.elapsed();
    println!("Atomic operations (1000 entities): {:?}", atomic_time);

    // Note: We don't add Mutex/RwLock benchmarks here to keep the example simple
    // In practice, Mutex/RwLock would be slower due to locking overhead

    println!("\n✓ Mutability pattern demonstration complete!");
    println!("\nKey Takeaways:");
    println!("  - Use AtomicU32/AtomicBool for simple lock-free updates");
    println!("  - Use Mutex<T> for exclusive access to complex state");
    println!("  - Use RwLock<T> for read-heavy workloads with occasional writes");
    println!("  - Always consider the read/write ratio when choosing synchronization primitives");
}
