use structecs::*;

// Test entities
#[derive(Debug, Extractable)]
pub struct SimpleEntity {
    pub name: String,
    pub value: i32,
}

#[derive(Debug, Extractable)]
pub struct Player {
    pub name: String,
    pub health: u32,
    pub level: u32,
}

#[derive(Debug, Extractable)]
pub struct Enemy {
    pub name: String,
    pub damage: u32,
}

#[derive(Debug, Extractable)]
pub struct Item {
    pub name: String,
    pub value: u32,
}

#[test]
fn test_world_creation() {
    let world = World::new();
    assert_eq!(world.entity_count(), 0);
    assert_eq!(world.archetype_count(), 0);
}

#[test]
fn test_add_single_entity() {
    let world = World::new();

    let entity = SimpleEntity {
        name: "Test".to_string(),
        value: 42,
    };

    let id = world.add_entity(entity);
    assert_eq!(world.entity_count(), 1);

    // Verify the entity can be queried
    let component = world.extract_component::<SimpleEntity>(&id);
    assert!(component.is_some());
}

#[test]
fn test_add_multiple_entities() {
    let world = World::new();

    for i in 0..10 {
        world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
            level: i,
        });
    }

    assert_eq!(world.entity_count(), 10);
}

#[test]
fn test_add_different_entity_types() {
    let world = World::new();

    world.add_entity(Player {
        name: "Alice".to_string(),
        health: 100,
        level: 5,
    });

    world.add_entity(Enemy {
        name: "Zombie".to_string(),
        damage: 50,
    });

    world.add_entity(Item {
        name: "Sword".to_string(),
        value: 100,
    });

    assert_eq!(world.entity_count(), 3);
    // Should have 3 different archetypes
    assert_eq!(world.archetype_count(), 3);
}

#[test]
fn test_remove_entity() {
    let world = World::new();

    let id = world.add_entity(Player {
        name: "Test".to_string(),
        health: 100,
        level: 1,
    });

    assert_eq!(world.entity_count(), 1);

    let removed = world.remove_entity(&id);
    assert!(removed);
    assert_eq!(world.entity_count(), 0);
}

#[test]
fn test_remove_nonexistent_entity() {
    let world = World::new();

    // Create a valid ID first, then remove twice
    let id = world.add_entity(Player {
        name: "Test".to_string(),
        health: 100,
        level: 1,
    });

    world.remove_entity(&id);

    // Second removal should fail
    let removed = world.remove_entity(&id);
    assert!(!removed);
}

#[test]
fn test_remove_multiple_entities() {
    let world = World::new();

    let mut ids = vec![];
    for i in 0..10 {
        let id = world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
            level: i,
        });
        ids.push(id);
    }

    // Remove half
    (0..5).for_each(|i| {
        world.remove_entity(&ids[i]);
    });

    assert_eq!(world.entity_count(), 5);
}

#[test]
fn test_extract_component() {
    let world = World::new();

    let id = world.add_entity(Player {
        name: "Alice".to_string(),
        health: 100,
        level: 5,
    });

    let player = world.extract_component::<Player>(&id);
    assert!(player.is_some());

    let player = player.unwrap();
    assert_eq!(player.name, "Alice");
    assert_eq!(player.health, 100);
    assert_eq!(player.level, 5);
}

#[test]
fn test_extract_component_nonexistent() {
    let world = World::new();

    // Create a valid ID, remove it, then try to extract
    let id = world.add_entity(Player {
        name: "Test".to_string(),
        health: 100,
        level: 1,
    });

    world.remove_entity(&id);

    let player = world.extract_component::<Player>(&id);
    assert!(player.is_none());
}

#[test]
fn test_query() {
    let world = World::new();

    for i in 0..10 {
        world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
            level: i,
        });
    }

    let mut count = 0;
    for (_id, player) in world.query::<Player>() {
        assert_eq!(player.health, 100);
        count += 1;
    }

    assert_eq!(count, 10);
}

#[test]
fn test_query_empty() {
    let world = World::new();

    let mut count = 0;
    for _ in world.query::<Player>() {
        count += 1;
    }

    assert_eq!(count, 0);
}

#[test]
fn test_query_multiple_types() {
    let world = World::new();

    for i in 0..5 {
        world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
            level: i,
        });
    }

    for i in 0..3 {
        world.add_entity(Enemy {
            name: format!("Enemy{}", i),
            damage: 50,
        });
    }

    let player_count = world.query::<Player>().len();
    let enemy_count = world.query::<Enemy>().len();

    assert_eq!(player_count, 5);
    assert_eq!(enemy_count, 3);
}

#[test]
fn test_large_entity_set() {
    let world = World::new();

    for i in 0..10_000 {
        world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
            level: i,
        });
    }

    assert_eq!(world.entity_count(), 10_000);

    let query_count = world.query::<Player>().len();
    assert_eq!(query_count, 10_000);
}

#[test]
fn test_mixed_operations() {
    let world = World::new();

    // Add entities
    let mut ids = vec![];
    for i in 0..100 {
        let id = world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
            level: i,
        });
        ids.push(id);
    }

    assert_eq!(world.entity_count(), 100);

    // Remove some
    (0..30).for_each(|i| {
        world.remove_entity(&ids[i]);
    });

    assert_eq!(world.entity_count(), 70);

    // Add more
    for i in 100..150 {
        world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
            level: i,
        });
    }

    assert_eq!(world.entity_count(), 120);

    // Verify query
    let count = world.query::<Player>().len();
    assert_eq!(count, 120);
}

#[test]
fn test_entity_data_extraction() {
    #[derive(Debug, Extractable)]
    pub struct TestEntity {
        pub name: String,
    }

    #[derive(Debug, Extractable)]
    #[extractable(entity)]
    pub struct TestPlayer {
        pub entity: TestEntity,
        pub health: u32,
    }

    let world = World::new();

    let id = world.add_entity(TestPlayer {
        entity: TestEntity {
            name: "Hero".to_string(),
        },
        health: 100,
    });

    // Extract nested component
    let entity = world.extract_component::<TestEntity>(&id);
    assert!(entity.is_some());
    assert_eq!(entity.unwrap().name, "Hero");

    // Extract full component
    let player = world.extract_component::<TestPlayer>(&id);
    assert!(player.is_some());
    assert_eq!(player.unwrap().health, 100);
}

#[test]
fn test_query_consistency() {
    let world = World::new();

    for i in 0..100 {
        world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
            level: i,
        });
    }

    // Run multiple queries and ensure consistent results
    for _ in 0..10 {
        let count = world.query::<Player>().len();
        assert_eq!(count, 100);
    }
}

#[test]
fn test_archetype_count() {
    let world = World::new();

    world.add_entity(Player {
        name: "Player".to_string(),
        health: 100,
        level: 1,
    });

    assert_eq!(world.archetype_count(), 1);

    world.add_entity(Enemy {
        name: "Enemy".to_string(),
        damage: 50,
    });

    assert_eq!(world.archetype_count(), 2);

    // Adding same type shouldn't increase archetype count
    world.add_entity(Player {
        name: "Player2".to_string(),
        health: 100,
        level: 2,
    });

    assert_eq!(world.archetype_count(), 2);
}
