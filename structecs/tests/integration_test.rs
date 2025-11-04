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
    assert!(component.is_ok());
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
    assert!(removed.is_ok());
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

    let _ = world.remove_entity(&id);

    // Second removal should fail
    let removed = world.remove_entity(&id);
    assert!(removed.is_err());
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
        let _ = world.remove_entity(&ids[i]);
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
    assert!(player.is_ok());

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

    let _ = world.remove_entity(&id);

    let player = world.extract_component::<Player>(&id);
    assert!(player.is_err());
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
        let _ = world.remove_entity(&ids[i]);
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
    assert!(entity.is_ok());
    assert_eq!(entity.unwrap().name, "Hero");

    // Extract full component
    let player = world.extract_component::<TestPlayer>(&id);
    assert!(player.is_ok());
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

#[test]
fn test_add_entities_batch() {
    let world = World::new();

    let entities: Vec<Player> = (0..10)
        .map(|i| Player {
            name: format!("Player{}", i),
            health: 100,
            level: i,
        })
        .collect();

    let ids = world.add_entities(entities);
    assert_eq!(ids.len(), 10);
    assert_eq!(world.entity_count(), 10);

    // Verify all entities can be queried
    let players = world.query::<Player>();
    assert_eq!(players.len(), 10);
}

#[test]
fn test_add_entities_empty() {
    let world = World::new();

    let entities: Vec<Player> = vec![];
    let ids = world.add_entities(entities);

    assert_eq!(ids.len(), 0);
    assert_eq!(world.entity_count(), 0);
}

#[test]
fn test_add_entities_large_batch() {
    let world = World::new();

    let entities: Vec<Player> = (0..10_000)
        .map(|i| Player {
            name: format!("Player{}", i),
            health: 100,
            level: i,
        })
        .collect();

    let ids = world.add_entities(entities);
    assert_eq!(ids.len(), 10_000);
    assert_eq!(world.entity_count(), 10_000);

    let players = world.query::<Player>();
    assert_eq!(players.len(), 10_000);
}

#[test]
fn test_remove_entities_batch() {
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

    // Remove first 50 in batch
    let result = world.remove_entities(&ids[0..50]);
    assert!(result.is_ok());
    assert_eq!(world.entity_count(), 50);

    // Verify remaining entities
    let players = world.query::<Player>();
    assert_eq!(players.len(), 50);
}

#[test]
fn test_remove_entities_empty() {
    let world = World::new();

    let ids: Vec<EntityId> = vec![];
    let result = world.remove_entities(&ids);
    assert!(result.is_ok());
}

#[test]
fn test_remove_entities_nonexistent() {
    let world = World::new();

    // Create IDs but don't add them
    let fake_ids = vec![
        EntityId::from_raw(9999),
        EntityId::from_raw(10000),
        EntityId::from_raw(10001),
    ];

    let result = world.remove_entities(&fake_ids);
    // All entities are nonexistent, so we should get an error
    assert!(result.is_err());
    if let Err(WorldError::PartialRemoval { succeeded, failed }) = result {
        assert_eq!(succeeded.len(), 0);
        assert_eq!(failed.len(), 3);
    }
}

#[test]
fn test_remove_entities_mixed() {
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

    // Remove some entities individually first
    let _ = world.remove_entity(&ids[0]);
    let _ = world.remove_entity(&ids[1]);

    // Now try to remove batch including already-removed ones
    let remove_ids = vec![ids[0], ids[1], ids[2], ids[3], ids[4]];
    let result = world.remove_entities(&remove_ids);

    // Should have partial success: 3 succeeded (ids[2], ids[3], ids[4]), 2 failed (ids[0], ids[1])
    assert!(result.is_err());
    if let Err(WorldError::PartialRemoval { succeeded, failed }) = result {
        assert_eq!(succeeded.len(), 3);
        assert_eq!(failed.len(), 2);
    }
    assert_eq!(world.entity_count(), 5);
}

#[test]
fn test_remove_entities_different_types() {
    let world = World::new();

    let mut player_ids = vec![];
    let mut enemy_ids = vec![];

    for i in 0..5 {
        player_ids.push(world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
            level: i,
        }));
        enemy_ids.push(world.add_entity(Enemy {
            name: format!("Enemy{}", i),
            damage: 50,
        }));
    }

    assert_eq!(world.entity_count(), 10);

    // Mix player and enemy IDs
    let mixed_ids = vec![player_ids[0], enemy_ids[0], player_ids[1], enemy_ids[1]];
    let result = world.remove_entities(&mixed_ids);

    // All entities exist, so should succeed
    assert!(result.is_ok());
    assert_eq!(world.entity_count(), 6);
}

#[test]
fn test_contains_entity() {
    let world = World::new();

    let id = world.add_entity(Player {
        name: "Test".to_string(),
        health: 100,
        level: 1,
    });

    assert!(world.contains_entity(&id));

    let _ = world.remove_entity(&id);
    assert!(!world.contains_entity(&id));

    // Non-existent entity
    let fake_id = EntityId::from_raw(99999);
    assert!(!world.contains_entity(&fake_id));
}

#[test]
fn test_world_clear() {
    let world = World::new();

    // Add multiple entity types
    for i in 0..50 {
        world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
            level: i,
        });
    }

    for i in 0..30 {
        world.add_entity(Enemy {
            name: format!("Enemy{}", i),
            damage: 50,
        });
    }

    assert_eq!(world.entity_count(), 80);
    assert_eq!(world.archetype_count(), 2);

    world.clear();

    assert_eq!(world.entity_count(), 0);
    assert_eq!(world.archetype_count(), 0);
}
