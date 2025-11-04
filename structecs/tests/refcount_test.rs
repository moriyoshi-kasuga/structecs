use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use structecs::*;

#[derive(Debug, Extractable)]
struct TestComponent {
    value: u64,
}

#[test]
fn test_basic_refcount() {
    let world = World::new();

    let id = world.add_entity(TestComponent { value: 42 });

    // Create multiple references
    let comp1 = world.extract_component::<TestComponent>(&id).unwrap();
    let comp2 = comp1.clone();
    let comp3 = comp2.clone();

    // All should point to same data
    assert_eq!(comp1.value, 42);
    assert_eq!(comp2.value, 42);
    assert_eq!(comp3.value, 42);

    // Drop references one by one
    drop(comp1);
    assert_eq!(comp2.value, 42); // Still accessible
    drop(comp2);
    assert_eq!(comp3.value, 42); // Still accessible
    drop(comp3);
    // Now data should be dropped
}

#[test]
fn test_refcount_with_world_removal() {
    static DROP_COUNT_LOCAL: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, Extractable)]
    struct DropCounter {
        id: usize,
    }

    impl Drop for DropCounter {
        fn drop(&mut self) {
            DROP_COUNT_LOCAL.fetch_add(1, Ordering::SeqCst);
        }
    }

    DROP_COUNT_LOCAL.store(0, Ordering::SeqCst);

    let world = World::new();
    let id = world.add_entity(DropCounter { id: 1 });

    // Extract reference
    let comp = world.extract_component::<DropCounter>(&id).unwrap();

    // Remove from world while reference exists
    world.remove_entity(&id).unwrap();

    // Drop should not have been called yet (reference still held)
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 0);

    // Still accessible via reference
    assert_eq!(comp.id, 1);

    // Drop the reference
    drop(comp);

    // Now drop should have been called
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 1);
}

#[test]
fn test_multiple_refs_world_removal() {
    static DROP_COUNT_LOCAL: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, Extractable)]
    struct DropCounter {
        id: usize,
    }

    impl Drop for DropCounter {
        fn drop(&mut self) {
            DROP_COUNT_LOCAL.fetch_add(1, Ordering::SeqCst);
        }
    }

    DROP_COUNT_LOCAL.store(0, Ordering::SeqCst);

    let world = World::new();
    let id = world.add_entity(DropCounter { id: 2 });

    // Create multiple references
    let comp1 = world.extract_component::<DropCounter>(&id).unwrap();
    let comp2 = comp1.clone();
    let comp3 = comp1.clone();

    // Remove from world
    world.remove_entity(&id).unwrap();

    // Drop should not be called yet
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 0);

    // Drop references one by one
    drop(comp1);
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 0);
    drop(comp2);
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 0);
    drop(comp3);

    // Now drop should be called
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 1);
}

#[test]
fn test_concurrent_clone() {
    let world = Arc::new(World::new());
    let id = world.add_entity(TestComponent { value: 123 });

    let comp = world.extract_component::<TestComponent>(&id).unwrap();
    let comp = Arc::new(comp);

    let mut handles = vec![];
    let barrier = Arc::new(Barrier::new(10));

    // Multiple threads cloning simultaneously
    for _ in 0..10 {
        let comp_clone = Arc::clone(&comp);
        let barrier_clone = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            barrier_clone.wait(); // Synchronize all threads

            let mut local_clones = vec![];
            for _ in 0..1000 {
                local_clones.push(comp_clone.clone());
            }

            // Verify all clones
            for c in &local_clones {
                assert_eq!(c.value, 123);
            }

            // Drop all clones
            drop(local_clones);
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Original should still be valid
    assert_eq!(comp.value, 123);
}

#[test]
fn test_concurrent_clone_and_drop() {
    static DROP_COUNT_LOCAL: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, Extractable)]
    struct DropCounter {
        id: usize,
    }

    impl Drop for DropCounter {
        fn drop(&mut self) {
            DROP_COUNT_LOCAL.fetch_add(1, Ordering::SeqCst);
        }
    }

    DROP_COUNT_LOCAL.store(0, Ordering::SeqCst);

    let world = Arc::new(World::new());
    let id = world.add_entity(DropCounter { id: 3 });

    let comp = world.extract_component::<DropCounter>(&id).unwrap();
    let comp = Arc::new(comp);

    let mut handles = vec![];
    let barrier = Arc::new(Barrier::new(20));

    // Half the threads clone, half the threads access
    for thread_id in 0..20 {
        let comp_clone = Arc::clone(&comp);
        let barrier_clone = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            barrier_clone.wait();

            if thread_id % 2 == 0 {
                // Clone operations
                let mut clones = vec![];
                for _ in 0..500 {
                    clones.push(comp_clone.clone());
                }
                // Let them drop
            } else {
                // Just access
                for _ in 0..500 {
                    let _ = comp_clone.id;
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Should not have been dropped yet
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 0);

    // Remove from world first
    world.remove_entity(&id).unwrap();
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 0);

    drop(comp);

    // Now should be dropped
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 1);
}

#[test]
fn test_refcount_with_query() {
    let world = World::new();

    // Add multiple entities
    for i in 0..100 {
        world.add_entity(TestComponent { value: i });
    }

    // Query creates references
    let results = world.query::<TestComponent>();

    // Collect all references
    let refs: Vec<_> = results.into_iter().map(|(_, comp)| comp).collect();

    assert_eq!(refs.len(), 100);

    // All should be valid (order not guaranteed)
    let mut values: Vec<_> = refs.iter().map(|c| c.value).collect();
    values.sort();
    for (i, &value) in values.iter().enumerate() {
        assert_eq!(value, i as u64);
    }

    // Drop half explicitly
    let (first_half, second_half) = refs.split_at(50);
    drop(first_half.to_vec());

    // Second half still valid
    for comp in second_half {
        assert!(comp.value < 100);
    }

    // Query again - should still work
    let results = world.query::<TestComponent>();
    assert_eq!(results.len(), 100);
}

#[test]
fn test_nested_extraction_refcount() {
    #[derive(Debug, Extractable)]
    struct CompA {
        value: u32,
    }

    #[derive(Debug, Extractable)]
    struct Entity {
        a: CompA,
        #[allow(dead_code)]
        b: u64,
    }

    let world = World::new();
    let id = world.add_entity(Entity {
        a: CompA { value: 42 },
        b: 100,
    });

    let entity = world.extract_component::<Entity>(&id).unwrap();

    // Create multiple references
    let ref1 = entity.clone();
    let ref2 = entity.clone();
    let ref3 = entity.clone();

    // All should work
    assert_eq!(entity.a.value, 42);
    assert_eq!(ref1.a.value, 42);
    assert_eq!(ref2.a.value, 42);
    assert_eq!(ref3.a.value, 42);

    // Drop original and one ref
    drop(entity);
    drop(ref1);

    // Others still work
    assert_eq!(ref2.a.value, 42);
    assert_eq!(ref3.a.value, 42);
}

#[test]
fn test_massive_clone_chain() {
    let world = World::new();
    let id = world.add_entity(TestComponent { value: 999 });

    let comp = world.extract_component::<TestComponent>(&id).unwrap();

    // Create a long chain of clones
    let mut clones = vec![comp];
    for _ in 0..10_000 {
        clones.push(clones.last().unwrap().clone());
    }

    // All should be valid
    for c in &clones {
        assert_eq!(c.value, 999);
    }

    // Drop all but one
    clones.truncate(1);

    // Last one should still be valid
    assert_eq!(clones[0].value, 999);
}

#[test]
fn test_refcount_zero_check() {
    static DROP_COUNT_LOCAL: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, Extractable)]
    struct DropCounter {
        id: usize,
    }

    impl Drop for DropCounter {
        fn drop(&mut self) {
            DROP_COUNT_LOCAL.fetch_add(1, Ordering::SeqCst);
        }
    }

    DROP_COUNT_LOCAL.store(0, Ordering::SeqCst);

    let world = World::new();

    // Create and destroy many entities
    for i in 0..1000 {
        let id = world.add_entity(DropCounter { id: i });
        let comp = world.extract_component::<DropCounter>(&id).unwrap();

        // Create some clones
        let clone1 = comp.clone();
        let clone2 = comp.clone();

        // Drop them all
        drop(comp);
        drop(clone1);
        drop(clone2);

        // Remove from world - should trigger final drop
        world.remove_entity(&id).unwrap();
    }

    // All should have been dropped
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 1000);
}

#[test]
fn test_refcount_with_world_drop() {
    static DROP_COUNT_LOCAL: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, Extractable)]
    struct DropCounter {
        id: usize,
    }

    impl Drop for DropCounter {
        fn drop(&mut self) {
            DROP_COUNT_LOCAL.fetch_add(1, Ordering::SeqCst);
        }
    }

    DROP_COUNT_LOCAL.store(0, Ordering::SeqCst);

    let comp = {
        let world = World::new();
        let id = world.add_entity(DropCounter { id: 5 });
        world.extract_component::<DropCounter>(&id).unwrap()
        // world drops here
    };

    // Should not have been dropped yet (comp still holds reference)
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 0);

    assert_eq!(comp.id, 5);

    drop(comp);

    // Now should be dropped
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 1);
}

#[test]
fn test_concurrent_refcount_stress() {
    static DROP_COUNT_LOCAL: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, Extractable)]
    struct DropCounter {
        id: usize,
    }

    impl Drop for DropCounter {
        fn drop(&mut self) {
            DROP_COUNT_LOCAL.fetch_add(1, Ordering::SeqCst);
        }
    }

    DROP_COUNT_LOCAL.store(0, Ordering::SeqCst);

    let world = Arc::new(World::new());
    let mut ids = vec![];

    // Create multiple entities
    for i in 0..100 {
        ids.push(world.add_entity(DropCounter { id: i }));
    }

    let ids = Arc::new(ids);
    let mut handles = vec![];

    // Many threads extracting and cloning randomly
    for _ in 0..20 {
        let world_clone = Arc::clone(&world);
        let ids_clone = Arc::clone(&ids);

        let handle = thread::spawn(move || {
            for _ in 0..1000 {
                let idx = simple_rand() % ids_clone.len();
                if let Ok(comp) = world_clone.extract_component::<DropCounter>(&ids_clone[idx]) {
                    let mut clones = vec![comp];
                    for _ in 0..10 {
                        clones.push(clones.last().unwrap().clone());
                    }
                    // Let them all drop
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Nothing should be dropped yet
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 0);

    // Remove all entities
    for id in ids.iter() {
        world.remove_entity(id).unwrap();
    }

    // All should be dropped now
    assert_eq!(DROP_COUNT_LOCAL.load(Ordering::SeqCst), 100);
}

// Simple pseudo-random number generator for testing
fn simple_rand() -> usize {
    use std::sync::atomic::AtomicU64;
    static SEED: AtomicU64 = AtomicU64::new(0x123456789ABCDEF0);

    let mut x = SEED.load(Ordering::Relaxed);
    if x == 0 {
        x = 0x123456789ABCDEF0;
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    SEED.store(x, Ordering::Relaxed);
    x as usize
}
