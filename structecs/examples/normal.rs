use structecs::*;

#[derive(Debug, Extractable)]
pub struct Entity {
    pub name: String,
}

// Example entity
#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Player {
    pub entity: Entity,
    pub health: u32,
}

fn main() {
    let mut world = World::default();

    let zombie = Entity {
        name: "Zombie".to_string(),
    };

    world.add_entity(zombie);

    let player = Player {
        entity: Entity {
            name: "Hero".to_string(),
        },
        health: 100,
    };

    world.add_entity(player);

    for (_, entity) in world.query::<Entity>() {
        println!("Entity: {:?}", *entity);
        if let Some(player) = entity.extract::<Player>() {
            println!("  As Player: {:?}", *player);
        }
    }
}
