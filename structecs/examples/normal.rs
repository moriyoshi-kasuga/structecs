use structecs::*;

#[derive(Debug, Extractable)]
pub struct Entity {
    pub name: String,
}

// Example entity with nested component
#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Player {
    pub entity: Entity,
    pub health: u32,
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Zombie {
    pub entity: Entity,
    pub damage: u32,
}

fn main() {
    let world = World::default();

    println!("=== Adding Entities ===");

    // Add some zombies
    for i in 0..3 {
        let zombie = Zombie {
            entity: Entity {
                name: format!("Zombie #{}", i),
            },
            damage: 10 + i * 5,
        };
        world.add_entity(zombie);
    }

    // Add some players
    for i in 0..2 {
        let player = Player {
            entity: Entity {
                name: format!("Player #{}", i),
            },
            health: 100,
        };
        world.add_entity(player);
    }

    println!("Total entities: {}", world.entity_count());
    println!("Total archetypes: {}", world.archetype_count());

    println!("\n=== Query All Entities (Iterator) ===");
    // Using iterator-based query (efficient, no allocation until snapshot)
    for (id, entity) in world.query::<Entity>() {
        println!("[{}] Entity: {:?}", id.id(), *entity);
    }

    println!("\n=== Query Players Only ===");
    for (id, player) in world.query::<Player>() {
        println!("[{}] Player: {:?}", id.id(), *player);
    }

    println!("\n=== Query Zombies Only ===");
    for (id, zombie) in world.query::<Zombie>() {
        println!("[{}] Zombie: {:?}", id.id(), *zombie);
    }

    println!("\n=== Extract Components ===");
    // Extract nested components from entities
    for (id, entity) in world.query::<Entity>() {
        print!("[{}] {}", id.id(), entity.name);

        // Try to extract as Player
        if let Some(player) = entity.extract::<Player>() {
            println!(" -> Player (health: {})", player.health);
        }
        // Try to extract as Zombie
        else if let Some(zombie) = entity.extract::<Zombie>() {
            println!(" -> Zombie (damage: {})", zombie.damage);
        } else {
            println!();
        }
    }

    println!("\n=== Performance Test ===");
    let start = std::time::Instant::now();

    // Add many entities
    for i in 0..10000 {
        let player = Player {
            entity: Entity {
                name: format!("Player {}", i),
            },
            health: 100,
        };
        world.add_entity(player);
    }

    let add_time = start.elapsed();
    println!("Added 10,000 entities in {:?}", add_time);

    // Query performance
    let start = std::time::Instant::now();
    let mut count = 0;
    for (_, _entity) in world.query::<Entity>() {
        count += 1;
    }
    let query_time = start.elapsed();
    println!("Queried {} entities in {:?}", count, query_time);

    // Specific type query
    let start = std::time::Instant::now();
    let mut count = 0;
    for (_, _player) in world.query::<Player>() {
        count += 1;
    }
    let player_query_time = start.elapsed();
    println!("Queried {} players in {:?}", count, player_query_time);
}
