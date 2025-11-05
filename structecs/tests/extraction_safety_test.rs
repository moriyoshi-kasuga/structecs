use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use structecs::*;

#[derive(Debug, Extractable)]
struct Component1 {
    value: i32,
}

#[derive(Debug, Extractable)]
struct Component2 {
    data: String,
}

#[derive(Debug, Extractable)]
struct Component3 {
    count: u64,
}

// Macro to create test-specific drop-tracked types
macro_rules! make_drop_tracked {
    ($name:ident, $counter:ident) => {
        static $counter: AtomicUsize = AtomicUsize::new(0);

        #[derive(Debug, Extractable)]
        struct $name {
            id: usize,
        }

        impl Drop for $name {
            fn drop(&mut self) {
                $counter.fetch_add(1, Ordering::SeqCst);
            }
        }
    };
}

#[test]
fn test_extract_nonexistent_component() {
    let world = World::new();
    let id = world.add_entity(Component1 { value: 42 });

    // Try to extract a component that doesn't exist
    let result = world.extract_component::<Component2>(&id);
    assert!(result.is_err());
}

#[test]
fn test_extract_from_invalid_entity() {
    let world = World::new();

    // Create a dummy entity ID
    let invalid_id = world.add_entity(Component1 { value: 42 });
    world.remove_entity(&invalid_id).unwrap();

    // Try to extract from removed entity
    let result = world.extract_component::<Component1>(&invalid_id);
    assert!(result.is_err());
}

#[test]
fn test_multiple_extractions_same_component() {
    let world = World::new();
    let id = world.add_entity(Component1 { value: 100 });

    // Extract multiple times
    let comp1 = world.extract_component::<Component1>(&id).unwrap();
    let comp2 = world.extract_component::<Component1>(&id).unwrap();
    let comp3 = world.extract_component::<Component1>(&id).unwrap();

    // All should have same value
    assert_eq!(comp1.value, 100);
    assert_eq!(comp2.value, 100);
    assert_eq!(comp3.value, 100);
}

#[test]
fn test_extract_after_component_dropped() {
    make_drop_tracked!(DropTracked1, DROP_COUNTER1);

    let world = World::new();
    let id = world.add_entity(DropTracked1 { id: 1 });

    {
        let _comp = world.extract_component::<DropTracked1>(&id).unwrap();
        // comp dropped here
    }

    // Should not be dropped yet (entity still exists)
    assert_eq!(DROP_COUNTER1.load(Ordering::SeqCst), 0);

    // Can extract again
    let comp2 = world.extract_component::<DropTracked1>(&id).unwrap();
    assert_eq!(comp2.id, 1);
}

#[test]
fn test_extract_component_with_mutation() {
    #[derive(Debug, Extractable)]
    struct MutableComponent {
        value: std::sync::Arc<std::sync::Mutex<i32>>,
    }

    let world = World::new();
    let shared_value = Arc::new(std::sync::Mutex::new(0));
    let id = world.add_entity(MutableComponent {
        value: shared_value.clone(),
    });

    let comp1 = world.extract_component::<MutableComponent>(&id).unwrap();
    let comp2 = world.extract_component::<MutableComponent>(&id).unwrap();

    // Mutate through comp1
    *comp1.value.lock().unwrap() = 42;

    // comp2 should see the change
    assert_eq!(*comp2.value.lock().unwrap(), 42);

    // Original should see the change
    assert_eq!(*shared_value.lock().unwrap(), 42);
}

#[test]
fn test_extract_concurrent_different_entities() {
    let world = Arc::new(World::new());
    let id1 = world.add_entity(Component1 { value: 1 });
    let id2 = world.add_entity(Component1 { value: 2 });

    let world1 = Arc::clone(&world);
    let world2 = Arc::clone(&world);

    let handle1 = thread::spawn(move || {
        for _ in 0..1000 {
            let comp = world1.extract_component::<Component1>(&id1).unwrap();
            assert_eq!(comp.value, 1);
        }
    });

    let handle2 = thread::spawn(move || {
        for _ in 0..1000 {
            let comp = world2.extract_component::<Component1>(&id2).unwrap();
            assert_eq!(comp.value, 2);
        }
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}

#[test]
fn test_extract_concurrent_same_component() {
    let world = Arc::new(World::new());
    let id = world.add_entity(Component1 { value: 999 });

    let mut handles = vec![];

    for _ in 0..10 {
        let world_clone = Arc::clone(&world);

        let handle = thread::spawn(move || {
            for _ in 0..1000 {
                let comp = world_clone.extract_component::<Component1>(&id).unwrap();
                assert_eq!(comp.value, 999);
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_extract_with_large_component() {
    #[derive(Debug, Extractable)]
    struct LargeComponent {
        data: Vec<u8>,
    }

    let world = World::new();
    let large_data: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();
    let id = world.add_entity(LargeComponent {
        data: large_data.clone(),
    });

    // Extract large component
    let comp = world.extract_component::<LargeComponent>(&id).unwrap();
    assert_eq!(comp.data.len(), 1_000_000);
    assert_eq!(comp.data, large_data);
}

#[test]
fn test_extract_zst_component() {
    #[derive(Debug, Extractable)]
    struct ZeroSized;

    let world = World::new();
    let id = world.add_entity(ZeroSized);

    // Extract ZST multiple times
    let _comp1 = world.extract_component::<ZeroSized>(&id).unwrap();
    let _comp2 = world.extract_component::<ZeroSized>(&id).unwrap();
    let _comp3 = world.extract_component::<ZeroSized>(&id).unwrap();

    // Should work without issues
}

#[test]
fn test_extract_after_world_operations() {
    let world = World::new();
    let id1 = world.add_entity(Component1 { value: 1 });
    let id2 = world.add_entity(Component1 { value: 2 });
    let id3 = world.add_entity(Component1 { value: 3 });

    // Extract from id1
    let comp1 = world.extract_component::<Component1>(&id1).unwrap();
    assert_eq!(comp1.value, 1);

    // Remove id2
    world.remove_entity(&id2).unwrap();

    // id1 should still be extractable
    let comp1_again = world.extract_component::<Component1>(&id1).unwrap();
    assert_eq!(comp1_again.value, 1);

    // id2 should not be extractable
    assert!(world.extract_component::<Component1>(&id2).is_err());

    // id3 should still be extractable
    let comp3 = world.extract_component::<Component1>(&id3).unwrap();
    assert_eq!(comp3.value, 3);
}

#[test]
fn test_extract_clone_independence() {
    make_drop_tracked!(DropTracked2, DROP_COUNTER2);

    let world = World::new();
    let id = world.add_entity(DropTracked2 { id: 42 });

    let comp1 = world.extract_component::<DropTracked2>(&id).unwrap();
    let comp2 = comp1.clone();

    // Drop comp1
    drop(comp1);

    // comp2 should still be valid
    assert_eq!(comp2.id, 42);

    // Should not be dropped yet
    assert_eq!(DROP_COUNTER2.load(Ordering::SeqCst), 0);

    drop(comp2);
    world.remove_entity(&id).unwrap();

    // Now should be dropped
    assert_eq!(DROP_COUNTER2.load(Ordering::SeqCst), 1);
}

#[test]
fn test_extract_stress() {
    let world = World::new();

    // Add many entities
    let mut ids = vec![];
    for i in 0..1000 {
        let id = world.add_entity(Component1 { value: i });
        ids.push(id);
    }

    // Extract all of them multiple times
    for _ in 0..10 {
        for (i, id) in ids.iter().enumerate() {
            let comp = world.extract_component::<Component1>(id).unwrap();
            assert_eq!(comp.value, i as i32);
        }
    }

    // Remove half
    for i in 0..500 {
        world.remove_entity(&ids[i]).unwrap();
    }

    // Remaining half should still be extractable
    for i in 500..1000 {
        let comp = world.extract_component::<Component1>(&ids[i]).unwrap();
        assert_eq!(comp.value, i as i32);
    }

    // Removed half should not be extractable
    for i in 0..500 {
        assert!(world.extract_component::<Component1>(&ids[i]).is_err());
    }
}

#[test]
fn test_extract_with_high_alignment() {
    #[repr(align(32))]
    #[derive(Debug, Extractable)]
    struct HighAlignComponent {
        value: u64,
    }

    let world = World::new();
    let id = world.add_entity(HighAlignComponent {
        value: 0x1234567890ABCDEF,
    });

    // Extract and verify
    let comp = world.extract_component::<HighAlignComponent>(&id).unwrap();
    assert_eq!(comp.value, 0x1234567890ABCDEF);

    // Verify alignment
    let ptr = &comp.value as *const u64;
    assert_eq!(ptr as usize % 32, 0, "Component not properly aligned");
}

#[test]
fn test_extract_mixed_lifetimes() {
    make_drop_tracked!(DropTracked3, DROP_COUNTER3);

    let world = World::new();
    let id = world.add_entity(DropTracked3 { id: 100 });

    let comp1 = world.extract_component::<DropTracked3>(&id).unwrap();

    {
        let comp2 = world.extract_component::<DropTracked3>(&id).unwrap();
        assert_eq!(comp2.id, 100);
        // comp2 dropped here
    }

    {
        let comp3 = world.extract_component::<DropTracked3>(&id).unwrap();
        assert_eq!(comp3.id, 100);
        // comp3 dropped here
    }

    // comp1 still valid
    assert_eq!(comp1.id, 100);

    // Should not be dropped yet
    assert_eq!(DROP_COUNTER3.load(Ordering::SeqCst), 0);

    drop(comp1);
    world.remove_entity(&id).unwrap();

    // Now should be dropped
    assert_eq!(DROP_COUNTER3.load(Ordering::SeqCst), 1);
}

#[test]
fn test_extract_with_query_interaction() {
    let world = World::new();
    let _id1 = world.add_entity(Component1 { value: 10 });
    let id2 = world.add_entity(Component1 { value: 20 });
    let _id3 = world.add_entity(Component1 { value: 30 });

    // Extract one entity
    let _extracted = world.extract_component::<Component1>(&id2).unwrap();

    // Query should still work and include all entities
    let mut count = 0;
    let mut values = vec![];
    for (_id, comp) in world.query::<Component1>() {
        count += 1;
        values.push(comp.value);
    }

    assert_eq!(count, 3);
    values.sort();
    assert_eq!(values, vec![10, 20, 30]);
}

#[test]
fn test_extract_concurrent_with_removals() {
    make_drop_tracked!(DropTracked4, DROP_COUNTER4);

    let world = Arc::new(World::new());

    // Add many entities
    let mut ids = vec![];
    for i in 0..100 {
        let id = world.add_entity(DropTracked4 { id: i });
        ids.push(id);
    }

    let ids = Arc::new(ids);
    let mut handles = vec![];

    // Thread 1: Extract components
    let world1 = Arc::clone(&world);
    let ids1 = Arc::clone(&ids);
    let handle1 = thread::spawn(move || {
        for _ in 0..100 {
            for id in ids1.iter() {
                let _ = world1.extract_component::<DropTracked4>(id);
            }
        }
    });
    handles.push(handle1);

    // Thread 2: Remove entities
    let world2 = Arc::clone(&world);
    let ids2 = Arc::clone(&ids);
    let handle2 = thread::spawn(move || {
        thread::sleep(std::time::Duration::from_millis(1));
        for id in ids2.iter().skip(50) {
            let _ = world2.remove_entity(id);
        }
    });
    handles.push(handle2);

    for handle in handles {
        handle.join().unwrap();
    }

    // At least 50 should be dropped
    assert!(DROP_COUNTER4.load(Ordering::SeqCst) >= 50);
}

#[test]
fn test_extract_type_safety() {
    let world = World::new();
    let id = world.add_entity(Component1 { value: 42 });

    // Can extract Component1
    assert!(world.extract_component::<Component1>(&id).is_ok());

    // Cannot extract Component2 (different type)
    assert!(world.extract_component::<Component2>(&id).is_err());

    // Cannot extract Component3 (different type)
    assert!(world.extract_component::<Component3>(&id).is_err());
}

#[test]
fn test_extract_maintains_refcount() {
    make_drop_tracked!(DropTracked5, DROP_COUNTER5);

    let world = World::new();
    let id = world.add_entity(DropTracked5 { id: 777 });

    // Create multiple extractions
    let comps: Vec<_> = (0..10)
        .map(|_| world.extract_component::<DropTracked5>(&id).unwrap())
        .collect();

    // All should be valid
    for comp in &comps {
        assert_eq!(comp.id, 777);
    }

    // Drop half
    drop(comps);

    // Should not be dropped yet (entity still exists)
    assert_eq!(DROP_COUNTER5.load(Ordering::SeqCst), 0);

    // Remove entity
    world.remove_entity(&id).unwrap();

    // Now should be dropped
    assert_eq!(DROP_COUNTER5.load(Ordering::SeqCst), 1);
}

#[test]
fn test_extract_returns_error_for_missing_entity() {
    let world = World::new();

    // Add and remove an entity
    let id = world.add_entity(Component1 { value: 42 });
    world.remove_entity(&id).unwrap();

    // Try to extract from removed entity
    assert!(world.extract_component::<Component1>(&id).is_err());

    // Try to extract different type
    let id2 = world.add_entity(Component2 {
        data: "test".to_string(),
    });
    assert!(world.extract_component::<Component1>(&id2).is_err());
}
