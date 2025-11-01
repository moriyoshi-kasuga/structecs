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
pub struct PlayerDeathed {
    pub is_dead: bool,
}

#[derive(Debug, Extractable)]
#[extractable(entity)]
pub struct Zombie {
    pub entity: Entity,
    pub damage: u32,
}

fn main() {
    let world = World::default();

    let player = Player {
        entity: Entity {
            name: "Hero".to_string(),
        },
        health: 100,
    };
    let player_id = world.add_entity(player);
    world.add_additional(&player_id, PlayerDeathed { is_dead: false });
    let zombie = Zombie {
        entity: Entity {
            name: "Walker".to_string(),
        },
        damage: 15,
    };
    world.add_entity(zombie);

    println!("=== Querying Entities ===");

    for (id, entity) in world.query_iter::<Entity>() {
        println!("Entity ID {}: {:?}", id.id(), *entity);

        if let Some(deathed) = world.extract_additional::<PlayerDeathed>(&id) {
            println!("  Deathed Status: {:?}", *deathed);
        }
    }
}
