#![allow(clippy::print_stdout, clippy::unwrap_used)]

use structecs::*;

#[derive(Debug, Extractable)]
pub struct Entity {
    pub death_handler: ComponentHandler<Entity, (), ()>,
    pub name: String,
}

// Example entity with nested component
#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Player {
    pub entity: Entity,
    pub health: u32,
    pub level: u32,
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Zombie {
    pub entity: Entity,
    pub damage: u32,
    pub is_baby: bool,
}

fn main() {
    println!("=== structecs Handler Example ===\n");

    let world = World::new();

    // Define a death handler function
    let death_handler = ComponentHandler::<Entity, (), ()>::new::<Entity>(|entity, ()| {
        println!("{} has died!", entity.name);
    });

    let player_death_handler = ComponentHandler::<Entity, (), ()>::new::<Player>(|player, ()| {
        println!(
            "Level {} player {} has perished!",
            player.level, player.entity.name
        );
    });

    // Create a player with the death handler
    world.add_entity(Player {
        entity: Entity {
            death_handler: player_death_handler,
            name: "Hero".to_string(),
        },
        health: 100,
        level: 5,
    });

    // Create a zombie with the death handler
    world.add_entity(Zombie {
        entity: Entity {
            death_handler: death_handler.clone(),
            name: "Undead Walker".to_string(),
        },
        damage: 15,
        is_baby: false,
    });

    println!("3. Simulating entity deaths...\n");

    for (id, entity) in world.query::<Entity>() {
        println!("Simulating death of entity ID {:?}...", id);
        entity.death_handler.call(&entity, ());
    }
}
