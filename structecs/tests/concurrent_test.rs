use std::sync::Arc;
use std::thread;
use std::time::Duration;
use structecs::*;

#[derive(Debug, Extractable)]
pub struct Player {
    pub name: String,
    pub health: i32,
}

#[derive(Debug, Extractable)]
pub struct Enemy {
    pub damage: i32,
}

#[test]
fn test_concurrent_inserts_10_threads() {
    let world = Arc::new(World::new());
    let mut handles = vec![];

    for thread_id in 0..10 {
        let world_clone = Arc::clone(&world);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                world_clone.add_entity(Player {
                    name: format!("Player_{}_{}", thread_id, i),
                    health: 100,
                });
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(world.entity_count(), 1000);
}

#[test]
fn test_concurrent_inserts_100_threads() {
    let world = Arc::new(World::new());
    let mut handles = vec![];

    for thread_id in 0..100 {
        let world_clone = Arc::clone(&world);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                world_clone.add_entity(Player {
                    name: format!("Player_{}_{}", thread_id, i),
                    health: 100,
                });
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // 100 threads * 100 entities = 10,000 entities
    assert_eq!(world.entity_count(), 10_000);
}

#[test]
fn test_concurrent_reads_while_writing() {
    let world = Arc::new(World::new());

    // Pre-populate
    for i in 0..1000 {
        world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
        });
    }

    let mut handles = vec![];

    // Writer thread
    let world_writer = Arc::clone(&world);
    let writer = thread::spawn(move || {
        for i in 1000..2000 {
            world_writer.add_entity(Player {
                name: format!("Player{}", i),
                health: 100,
            });
            thread::sleep(Duration::from_micros(10));
        }
    });
    handles.push(writer);

    // Multiple reader threads
    for _ in 0..10 {
        let world_reader = Arc::clone(&world);
        let reader = thread::spawn(move || {
            for _ in 0..100 {
                let count = world_reader.query_iter::<Player>().count();
                assert!((1000..=2000).contains(&count));
                thread::sleep(Duration::from_micros(10));
            }
        });
        handles.push(reader);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(world.entity_count(), 2000);
}

#[test]
fn test_concurrent_insert_and_remove() {
    let world = Arc::new(World::new());

    // Pre-populate
    let mut ids = vec![];
    for i in 0..1000 {
        let id = world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
        });
        ids.push(id);
    }

    let ids = Arc::new(ids);

    let mut handles = vec![];

    // Inserter thread
    let world_inserter = Arc::clone(&world);
    let inserter = thread::spawn(move || {
        for i in 1000..2000 {
            world_inserter.add_entity(Player {
                name: format!("Player{}", i),
                health: 100,
            });
        }
    });
    handles.push(inserter);

    // Remover thread
    let world_remover = Arc::clone(&world);
    let ids_remover = Arc::clone(&ids);
    let remover = thread::spawn(move || {
        for i in 0..500 {
            world_remover.remove_entity(&ids_remover[i]);
        }
    });
    handles.push(remover);

    for handle in handles {
        handle.join().unwrap();
    }

    // Should have 500 (remaining from 0-999) + 1000 (1000-1999) = 1500
    assert_eq!(world.entity_count(), 1500);
}

#[test]
fn test_concurrent_mixed_operations() {
    let world = Arc::new(World::new());

    // Pre-populate
    let mut ids = vec![];
    for i in 0..1000 {
        let id = world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
        });
        ids.push(id);
    }

    let ids = Arc::new(ids);

    let mut handles = vec![];

    // Inserters
    for thread_id in 0..10 {
        let world_clone = Arc::clone(&world);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                world_clone.add_entity(Player {
                    name: format!("Player_{}_{}", thread_id, i),
                    health: 100,
                });
            }
        });
        handles.push(handle);
    }

    // Removers
    for thread_id in 0..5 {
        let world_clone = Arc::clone(&world);
        let ids_clone = Arc::clone(&ids);
        let handle = thread::spawn(move || {
            for i in 0..50 {
                let idx = thread_id * 50 + i;
                world_clone.remove_entity(&ids_clone[idx]);
            }
        });
        handles.push(handle);
    }

    // Readers
    for _ in 0..10 {
        let world_clone = Arc::clone(&world);
        let handle = thread::spawn(move || {
            for _ in 0..50 {
                let _count = world_clone.query_iter::<Player>().count();
            }
        });
        handles.push(handle);
    }

    // Extractors
    for _ in 0..10 {
        let world_clone = Arc::clone(&world);
        let ids_clone = Arc::clone(&ids);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                let _player = world_clone.extract_component::<Player>(&ids_clone[i]);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // 1000 initial - 250 removed + 1000 inserted = 1750
    assert_eq!(world.entity_count(), 1750);
}

#[test]
fn test_concurrent_multiple_entity_types() {
    let world = Arc::new(World::new());
    let mut handles = vec![];

    // Insert Players
    for thread_id in 0..10 {
        let world_clone = Arc::clone(&world);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                world_clone.add_entity(Player {
                    name: format!("Player_{}_{}", thread_id, i),
                    health: 100,
                });
            }
        });
        handles.push(handle);
    }

    // Insert Enemies
    for _thread_id in 0..10 {
        let world_clone = Arc::clone(&world);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                world_clone.add_entity(Enemy { damage: 50 + i });
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(world.query_iter::<Player>().count(), 1000);
    assert_eq!(world.query_iter::<Enemy>().count(), 1000);
    assert_eq!(world.entity_count(), 2000);
}

#[test]
fn test_concurrent_extract_operations() {
    let world = Arc::new(World::new());

    // Pre-populate
    let mut ids = vec![];
    for i in 0..1000 {
        let id = world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
        });
        ids.push(id);
    }

    let ids = Arc::new(ids);

    let mut handles = vec![];

    for thread_id in 0..20 {
        let world_clone = Arc::clone(&world);
        let ids_clone = Arc::clone(&ids);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                let idx = (thread_id * 100 + i) % 1000;
                let player = world_clone.extract_component::<Player>(&ids_clone[idx]);
                assert!(player.is_some());
                assert_eq!(player.unwrap().health, 100);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_concurrent_updates() {
    let world = Arc::new(World::new());

    // Pre-populate
    let mut ids = vec![];
    for i in 0..100 {
        let id = world.add_entity(Player {
            name: format!("Player{}", i),
            health: 100,
        });
        ids.push(id);
    }

    let ids = Arc::new(ids);

    let mut handles = vec![];

    // Multiple threads updating the same entities
    for thread_id in 0..10 {
        let world_clone = Arc::clone(&world);
        let ids_clone = Arc::clone(&ids);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                // Remove and reinsert with updated health
                world_clone.remove_entity(&ids_clone[i]);
                world_clone.add_entity(Player {
                    name: format!("Player{}", i),
                    health: 100 - thread_id,
                });
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Should have at least 100 entities (some might have been re-added)
    assert!(world.entity_count() >= 100);
}

#[test]
#[ignore = "Stress test - may take longer to run"]
fn test_stress_100_threads_heavy_load() {
    let world = Arc::new(World::new());
    let mut handles = vec![];

    for thread_id in 0..100 {
        let world_clone = Arc::clone(&world);
        let handle = thread::spawn(move || {
            // Each thread does mixed operations
            let mut local_ids = vec![];
            for i in 0..50 {
                // Insert
                let id = world_clone.add_entity(Player {
                    name: format!("Player_{}_{}", thread_id, i),
                    health: 100,
                });
                local_ids.push(id);

                // Extract
                let _player = world_clone.extract_component::<Player>(&id);

                // Query (partial)
                let _count = world_clone.query_iter::<Player>().take(10).count();

                // Update (remove + reinsert)
                if i % 2 == 0 {
                    world_clone.remove_entity(&id);
                    let new_id = world_clone.add_entity(Player {
                        name: format!("Player_{}_{}", thread_id, i),
                        health: 80,
                    });
                    local_ids[i] = new_id;
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // 100 threads * 50 entities = 5000 entities
    assert_eq!(world.entity_count(), 5000);
}

#[test]
fn test_no_data_races() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let world = Arc::new(World::new());
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for thread_id in 0..50 {
        let world_clone = Arc::clone(&world);
        let counter_clone = Arc::clone(&counter);

        let handle = thread::spawn(move || {
            for i in 0..100 {
                world_clone.add_entity(Player {
                    name: format!("Player_{}_{}", thread_id, i),
                    health: 100,
                });
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let inserted_count = counter.load(Ordering::SeqCst);
    let world_count = world.entity_count();

    assert_eq!(inserted_count, 5000);
    assert_eq!(world_count, 5000);
}
