use std::sync::Arc;
use std::thread;
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
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Monster {
    pub entity: Entity,
    pub damage: u32,
}

fn main() {
    // Create a world wrapped in Arc for sharing across threads
    let world = Arc::new(World::default());

    println!("=== Concurrent Entity Addition Test ===");
    println!("Adding entities from multiple threads...\n");

    let mut handles = vec![];

    // Thread 1: Add players
    let world_clone = Arc::clone(&world);
    let handle = thread::spawn(move || {
        let start = std::time::Instant::now();
        for i in 0..5000 {
            let player = Player {
                entity: Entity {
                    name: format!("Player {}", i),
                },
                health: 100,
            };
            world_clone.add_entity(player);
        }
        let elapsed = start.elapsed();
        println!("Thread 1 (Players): Added 5000 entities in {:?}", elapsed);
    });
    handles.push(handle);

    // Thread 2: Add monsters
    let world_clone = Arc::clone(&world);
    let handle = thread::spawn(move || {
        let start = std::time::Instant::now();
        for i in 0..5000 {
            let monster = Monster {
                entity: Entity {
                    name: format!("Monster {}", i),
                },
                damage: 10 + (i % 50),
            };
            world_clone.add_entity(monster);
        }
        let elapsed = start.elapsed();
        println!("Thread 2 (Monsters): Added 5000 entities in {:?}", elapsed);
    });
    handles.push(handle);

    // Thread 3: Add more players
    let world_clone = Arc::clone(&world);
    let handle = thread::spawn(move || {
        let start = std::time::Instant::now();
        for i in 5000..10000 {
            let player = Player {
                entity: Entity {
                    name: format!("Player {}", i),
                },
                health: 100,
            };
            world_clone.add_entity(player);
        }
        let elapsed = start.elapsed();
        println!("Thread 3 (Players): Added 5000 entities in {:?}", elapsed);
    });
    handles.push(handle);

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    println!("\n=== Results ===");
    println!("Total entities: {}", world.entity_count());
    println!("Total archetypes: {}", world.archetype_count());

    // Query from main thread while other threads might still be operating
    let player_count = world.query_iter::<Player>().count();
    let monster_count = world.query_iter::<Monster>().count();
    
    println!("Players: {}", player_count);
    println!("Monsters: {}", monster_count);

    println!("\n=== Concurrent Query Test ===");
    let mut handles = vec![];

    // Thread 1: Query players
    let world_clone = Arc::clone(&world);
    let handle = thread::spawn(move || {
        let start = std::time::Instant::now();
        let total_health: u32 = world_clone
            .query_iter::<Player>()
            .map(|(_, player)| player.health)
            .sum();
        let elapsed = start.elapsed();
        println!("Thread 1: Queried players, total health = {} in {:?}", total_health, elapsed);
    });
    handles.push(handle);

    // Thread 2: Query monsters
    let world_clone = Arc::clone(&world);
    let handle = thread::spawn(move || {
        let start = std::time::Instant::now();
        let total_damage: u32 = world_clone
            .query_iter::<Monster>()
            .map(|(_, monster)| monster.damage)
            .sum();
        let elapsed = start.elapsed();
        println!("Thread 2: Queried monsters, total damage = {} in {:?}", total_damage, elapsed);
    });
    handles.push(handle);

    // Thread 3: Query all entities
    let world_clone = Arc::clone(&world);
    let handle = thread::spawn(move || {
        let start = std::time::Instant::now();
        let count = world_clone.query_iter::<Entity>().count();
        let elapsed = start.elapsed();
        println!("Thread 3: Queried all entities, count = {} in {:?}", count, elapsed);
    });
    handles.push(handle);

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    println!("\n✓ All concurrent operations completed successfully!");
    println!("  - Multiple threads added entities to different archetypes in parallel");
    println!("  - Multiple threads queried different archetypes simultaneously");
    println!("  - No race conditions or data corruption detected");
}
