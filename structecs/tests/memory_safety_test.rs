use std::sync::Arc;
use std::thread;
use structecs::*;

#[derive(Debug, Extractable)]
pub struct TestEntity {
    pub data: Vec<u8>,
}

#[derive(Debug, Extractable)]
pub struct LargeEntity {
    pub data: Box<[u8; 1024]>, // 1KB per entity
}

#[test]
fn test_memory_cleanup_after_remove() {
    let world = World::new();

    // Insert entities
    let mut ids = vec![];
    for _ in 0..1000 {
        let id = world.add_entity(TestEntity {
            data: vec![0u8; 1024], // 1KB each
        });
        ids.push(id);
    }

    assert_eq!(world.entity_count(), 1000);

    // Remove all entities
    for id in &ids {
        let removed = world.remove_entity(id);
        assert!(removed);
    }

    assert_eq!(world.entity_count(), 0);

    // Re-insert to verify cleanup happened
    for _ in 0..1000 {
        world.add_entity(TestEntity {
            data: vec![1u8; 1024],
        });
    }

    assert_eq!(world.entity_count(), 1000);
}

#[test]
fn test_no_memory_leak_with_updates() {
    let world = World::new();

    // Repeatedly update entities (remove + reinsert)
    let mut ids = vec![];
    for _ in 0..500 {
        let id = world.add_entity(TestEntity {
            data: vec![0u8; 2048], // 2KB each
        });
        ids.push(id);
    }
    for _ in 0..10 {
        for id in &mut ids {
            world.remove_entity(id);
            let new_id = world.add_entity(TestEntity {
                data: vec![1u8; 2048],
            });
            *id = new_id;
        }
    }
    assert_eq!(world.entity_count(), 500);
}

#[test]
fn test_large_entity_lifecycle() {
    let world = World::new();

    // Insert large entities
    let mut ids = vec![];
    for i in 0..100 {
        let id = world.add_entity(LargeEntity {
            data: Box::new([i as u8; 1024]),
        });
        ids.push(id);
    }

    // Query and verify
    let count = world.query_iter::<LargeEntity>().count();
    assert_eq!(count, 100);

    // Remove half
    (0..50).for_each(|i| {
        world.remove_entity(&ids[i]);
    });

    assert_eq!(world.entity_count(), 50);
}

#[test]
fn test_concurrent_memory_safety() {
    let world = Arc::new(World::new());
    let mut handles = vec![];

    // Multiple threads inserting and removing
    for thread_id in 0..10 {
        let world_clone = Arc::clone(&world);
        let handle = thread::spawn(move || {
            let mut local_ids = vec![];
            for iteration in 0..100 {
                // Insert
                let id = world_clone.add_entity(TestEntity {
                    data: vec![thread_id as u8; 512],
                });
                local_ids.push(id);

                // Remove previous iteration
                if iteration > 0 {
                    world_clone.remove_entity(&local_ids[iteration - 1]);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Should have 10 entities remaining (last from each thread)
    assert_eq!(world.entity_count(), 10);
}

#[test]
fn test_drop_behavior() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, Extractable)]
    struct DropCounter {
        _value: u32,
    }

    impl Drop for DropCounter {
        fn drop(&mut self) {
            DROP_COUNT.fetch_add(1, Ordering::Release);
        }
    }

    // Reset counter
    DROP_COUNT.store(0, Ordering::SeqCst);

    let mut acquirable_holders: Vec<Acquirable<DropCounter>> = vec![];

    {
        let world = World::new();

        // Insert 100 entities
        let mut ids = vec![];
        for i in 0..100 {
            let id = world.add_entity(DropCounter { _value: i });
            ids.push(id);
        }

        // Remove 50 entities
        (0..50).for_each(|i| {
            world.remove_entity(&ids[i]);
        });

        (50..75).for_each(|i| {
            let holder = world.extract_component::<DropCounter>(&ids[i]).unwrap();
            acquirable_holders.push(holder);
        });

        // World goes out of scope here
    }

    let drop_count = DROP_COUNT.load(Ordering::Acquire);
    // 50 removed + 25 held = 75 drops expected
    assert_eq!(drop_count, 75);
}

#[test]
fn test_query_iterator_safety() {
    let world = World::new();

    // Insert entities
    for i in 0..1000 {
        world.add_entity(TestEntity {
            data: vec![i as u8; 128],
        });
    }

    // Create multiple iterators
    let iter1 = world.query_iter::<TestEntity>();
    let iter2 = world.query_iter::<TestEntity>();
    let iter3 = world.query_iter::<TestEntity>();

    // Consume iterators
    assert_eq!(iter1.count(), 1000);
    assert_eq!(iter2.count(), 1000);
    assert_eq!(iter3.count(), 1000);

    // Original data should still be accessible
    assert_eq!(world.query_iter::<TestEntity>().count(), 1000);
}

#[test]
fn test_parallel_query_memory_safety() {
    use rayon::prelude::*;

    let world = World::new();

    // Insert large number of entities
    let mut ids = vec![];
    for i in 0..10_000 {
        let id = world.add_entity(TestEntity {
            data: vec![i as u8; 256],
        });
        ids.push(id);
    }

    // Run parallel query multiple times
    for _ in 0..10 {
        let count: usize = world.par_query_iter::<TestEntity>().count();

        assert_eq!(count, 10_000);
    }
}

#[test]
fn test_empty_archetype_cleanup() {
    let world = World::new();

    // Insert and remove multiple times
    for iteration in 0..10 {
        let mut ids = vec![];
        for _ in 0..100 {
            let id = world.add_entity(TestEntity {
                data: vec![iteration as u8; 64],
            });
            ids.push(id);
        }

        for id in &ids {
            world.remove_entity(id);
        }

        // Verify empty
        assert_eq!(world.entity_count(), 0);
    }
}

#[test]
fn test_massive_insertion_and_removal() {
    let world = World::new();
    let entity_count = 50_000;

    // Insert massive number of entities
    let mut ids = vec![];
    for i in 0..entity_count {
        let id = world.add_entity(TestEntity {
            data: vec![(i % 256) as u8; 128],
        });
        ids.push(id);
    }

    assert_eq!(world.entity_count(), entity_count as usize);

    // Remove all
    for id in &ids {
        world.remove_entity(id);
    }

    assert_eq!(world.entity_count(), 0);

    // Should be able to insert again without issues
    for i in 0..1000 {
        world.add_entity(TestEntity {
            data: vec![i as u8; 128],
        });
    }

    assert_eq!(world.entity_count(), 1000);
}

#[test]
fn test_concurrent_extract_memory_safety() {
    let world = Arc::new(World::new());

    // Pre-populate
    let mut ids = vec![];
    for i in 0..1000 {
        let id = world.add_entity(TestEntity {
            data: vec![i as u8; 256],
        });
        ids.push(id);
    }

    let ids = Arc::new(ids);

    let mut handles = vec![];

    for thread_id in 0..20 {
        let world_clone = Arc::clone(&world);
        let ids_clone = Arc::clone(&ids);
        let handle = thread::spawn(move || {
            for _ in 0..1000 {
                let idx = (thread_id * 7) % 1000;
                let entity = world_clone.extract_component::<TestEntity>(&ids_clone[idx]);
                if let Some(entity) = entity {
                    assert!(!entity.data.is_empty());
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
