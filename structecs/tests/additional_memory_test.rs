use structecs::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

#[derive(Debug, Extractable)]
struct BaseComponent {
    value: u32,
}

#[derive(Debug, Extractable)]
struct Additional1 {
    data: u64,
}

#[derive(Debug, Extractable)]
struct Additional2 {
    text: String,
}

#[derive(Debug, Extractable)]
struct Additional3 {
    numbers: Vec<i32>,
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
fn test_add_single_additional() {
    let world = World::new();
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp = world.extract_component::<BaseComponent>(&id).unwrap();
    
    // Add additional component
    comp.add_additional(Additional1 { data: 123 });
    
    // Extract additional
    let additional = comp.extract_additional::<Additional1>().unwrap();
    assert_eq!(additional.data, 123);
}

#[test]
fn test_add_multiple_additional_types() {
    let world = World::new();
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp = world.extract_component::<BaseComponent>(&id).unwrap();
    
    // Add multiple different types
    comp.add_additional(Additional1 { data: 111 });
    comp.add_additional(Additional2 { text: "hello".to_string() });
    comp.add_additional(Additional3 { numbers: vec![1, 2, 3] });
    
    // Extract all
    let a1 = comp.extract_additional::<Additional1>().unwrap();
    let a2 = comp.extract_additional::<Additional2>().unwrap();
    let a3 = comp.extract_additional::<Additional3>().unwrap();
    
    assert_eq!(a1.data, 111);
    assert_eq!(a2.text, "hello");
    assert_eq!(a3.numbers, vec![1, 2, 3]);
}

#[test]
fn test_replace_additional_same_type() {
    make_drop_tracked!(DropTracked1, DROP_COUNTER1);
    
    let world = World::new();
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp = world.extract_component::<BaseComponent>(&id).unwrap();
    
    // Add first additional
    comp.add_additional(DropTracked1 { id: 1 });
    
    // First should not be dropped yet
    assert_eq!(DROP_COUNTER1.load(Ordering::SeqCst), 0);
    
    // Add second with same type (should replace)
    comp.add_additional(DropTracked1 { id: 2 });
    
    // First should be dropped now
    assert_eq!(DROP_COUNTER1.load(Ordering::SeqCst), 1);
    
    // Extract should get the second one
    let tracked = comp.extract_additional::<DropTracked1>().unwrap();
    assert_eq!(tracked.id, 2);
    
    // Clean up
    drop(tracked);
    drop(comp);
    world.remove_entity(&id).unwrap();
    
    // Second should be dropped
    assert_eq!(DROP_COUNTER1.load(Ordering::SeqCst), 2);
}

#[test]
fn test_additional_lifecycle() {
    make_drop_tracked!(DropTracked2, DROP_COUNTER2);
    
    let world = World::new();
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp = world.extract_component::<BaseComponent>(&id).unwrap();
    
    // Add additional
    comp.add_additional(DropTracked2 { id: 1 });
    
    // Extract it
    let additional = comp.extract_additional::<DropTracked2>().unwrap();
    assert_eq!(additional.id, 1);
    
    // Drop the reference
    drop(additional);
    
    // Should not be dropped yet (comp still holds it)
    assert_eq!(DROP_COUNTER2.load(Ordering::SeqCst), 0);
    
    // Extract again (should work)
    let additional2 = comp.extract_additional::<DropTracked2>().unwrap();
    assert_eq!(additional2.id, 1);
    
    drop(additional2);
    drop(comp);
    world.remove_entity(&id).unwrap();
    
    // Now should be dropped
    assert_eq!(DROP_COUNTER2.load(Ordering::SeqCst), 1);
}

#[test]
fn test_remove_additional() {
    make_drop_tracked!(DropTracked3, DROP_COUNTER3);
    
    let world = World::new();
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp = world.extract_component::<BaseComponent>(&id).unwrap();
    
    // Add additional
    comp.add_additional(DropTracked3 { id: 1 });
    
    // Remove it
    let removed = comp.remove_additional::<DropTracked3>();
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().id, 1);
    
    // Should not be able to extract again
    let none = comp.extract_additional::<DropTracked3>();
    assert!(none.is_none());
    
    // Should not be dropped yet
    assert_eq!(DROP_COUNTER3.load(Ordering::SeqCst), 0);
}

#[test]
fn test_additional_with_multiple_refs() {
    let world = World::new();
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp1 = world.extract_component::<BaseComponent>(&id).unwrap();
    let comp2 = comp1.clone();
    let comp3 = comp1.clone();
    
    // Add additional via first reference
    comp1.add_additional(Additional1 { data: 999 });
    
    // Should be accessible via all references
    assert_eq!(comp2.extract_additional::<Additional1>().unwrap().data, 999);
    assert_eq!(comp3.extract_additional::<Additional1>().unwrap().data, 999);
}

#[test]
fn test_additional_persists_across_extractions() {
    let world = World::new();
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp1 = world.extract_component::<BaseComponent>(&id).unwrap();
    
    // Add additional
    comp1.add_additional(Additional1 { data: 555 });
    
    drop(comp1);
    
    // Extract component again
    let comp2 = world.extract_component::<BaseComponent>(&id).unwrap();
    
    // Additional should still be there
    let additional = comp2.extract_additional::<Additional1>().unwrap();
    assert_eq!(additional.data, 555);
}

#[test]
fn test_many_additionals_same_type() {
    make_drop_tracked!(DropTracked4, DROP_COUNTER4);
    
    let world = World::new();
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp = world.extract_component::<BaseComponent>(&id).unwrap();
    
    // Add and replace many times
    for i in 0..100 {
        comp.add_additional(DropTracked4 { id: i });
        
        // Previous should be dropped (except for first iteration)
        if i > 0 {
            assert_eq!(DROP_COUNTER4.load(Ordering::SeqCst), i);
        }
    }
    
    // Should have the last one
    let tracked = comp.extract_additional::<DropTracked4>().unwrap();
    assert_eq!(tracked.id, 99);
    
    // 99 drops so far
    assert_eq!(DROP_COUNTER4.load(Ordering::SeqCst), 99);
    
    drop(tracked);
    drop(comp);
    world.remove_entity(&id).unwrap();
    
    // Now all 100 should be dropped
    assert_eq!(DROP_COUNTER4.load(Ordering::SeqCst), 100);
}

#[test]
fn test_many_different_additional_types() {
    let world = World::new();
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp = world.extract_component::<BaseComponent>(&id).unwrap();
    
    // Add 3 different types multiple times
    for i in 0..10 {
        comp.add_additional(Additional1 { data: i as u64 });
        comp.add_additional(Additional2 { text: format!("iteration_{}", i) });
        comp.add_additional(Additional3 { numbers: vec![i, i + 1, i + 2] });
    }
    
    // Should have the last values
    let a1 = comp.extract_additional::<Additional1>().unwrap();
    let a2 = comp.extract_additional::<Additional2>().unwrap();
    let a3 = comp.extract_additional::<Additional3>().unwrap();
    
    assert_eq!(a1.data, 9);
    assert_eq!(a2.text, "iteration_9");
    assert_eq!(a3.numbers, vec![9, 10, 11]);
}

#[test]
fn test_concurrent_additional_add() {
    let world = Arc::new(World::new());
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp = world.extract_component::<BaseComponent>(&id).unwrap();
    let comp = Arc::new(comp);
    
    let mut handles = vec![];
    
    // Multiple threads adding different additionals
    for thread_id in 0..10 {
        let comp_clone = Arc::clone(&comp);
        
        let handle = thread::spawn(move || {
            for i in 0..100 {
                // Each thread adds its own type pattern
                if thread_id % 3 == 0 {
                    comp_clone.add_additional(Additional1 { data: i as u64 });
                } else if thread_id % 3 == 1 {
                    comp_clone.add_additional(Additional2 { text: format!("{}", i) });
                } else {
                    comp_clone.add_additional(Additional3 { numbers: vec![i] });
                }
            }
        });
        
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    // All types should be present
    assert!(comp.extract_additional::<Additional1>().is_some());
    assert!(comp.extract_additional::<Additional2>().is_some());
    assert!(comp.extract_additional::<Additional3>().is_some());
}

#[test]
fn test_concurrent_additional_extract() {
    let world = Arc::new(World::new());
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp = world.extract_component::<BaseComponent>(&id).unwrap();
    
    // Add additionals
    comp.add_additional(Additional1 { data: 777 });
    comp.add_additional(Additional2 { text: "concurrent".to_string() });
    
    let comp = Arc::new(comp);
    let mut handles = vec![];
    
    // Multiple threads extracting simultaneously
    for _ in 0..20 {
        let comp_clone = Arc::clone(&comp);
        
        let handle = thread::spawn(move || {
            for _ in 0..1000 {
                let a1 = comp_clone.extract_additional::<Additional1>();
                let a2 = comp_clone.extract_additional::<Additional2>();
                
                if let Some(a1) = a1 {
                    assert_eq!(a1.data, 777);
                }
                if let Some(a2) = a2 {
                    assert_eq!(a2.text, "concurrent");
                }
            }
        });
        
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_additional_with_large_data() {
    let world = World::new();
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp = world.extract_component::<BaseComponent>(&id).unwrap();
    
    // Add large additional
    let large_vec: Vec<i32> = (0..10_000).collect();
    comp.add_additional(Additional3 { numbers: large_vec.clone() });
    
    // Extract and verify
    let extracted = comp.extract_additional::<Additional3>().unwrap();
    assert_eq!(extracted.numbers.len(), 10_000);
    assert_eq!(extracted.numbers, large_vec);
}

#[test]
fn test_additional_drop_on_world_drop() {
    make_drop_tracked!(DropTracked5, DROP_COUNTER5);
    
    {
        let world = World::new();
        let id = world.add_entity(BaseComponent { value: 42 });
        
        let comp = world.extract_component::<BaseComponent>(&id).unwrap();
        comp.add_additional(DropTracked5 { id: 1 });
        
        // Drop everything
        drop(comp);
        // world drops here
    }
    
    // Should be dropped
    assert_eq!(DROP_COUNTER5.load(Ordering::SeqCst), 1);
}

#[test]
fn test_additional_remove_and_readd() {
    let world = World::new();
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp = world.extract_component::<BaseComponent>(&id).unwrap();
    
    // Add, remove, add again cycle
    for i in 0..10 {
        comp.add_additional(Additional1 { data: i as u64 });
        let extracted = comp.extract_additional::<Additional1>().unwrap();
        assert_eq!(extracted.data, i as u64);
        
        let removed = comp.remove_additional::<Additional1>();
        assert!(removed.is_some());
        
        assert!(comp.extract_additional::<Additional1>().is_none());
    }
}

#[test]
fn test_additional_memory_stress() {
    make_drop_tracked!(DropTracked6, DROP_COUNTER6);
    
    let world = World::new();
    
    // Create many entities with additionals
    let mut ids = vec![];
    for i in 0..1000 {
        let id = world.add_entity(BaseComponent { value: i });
        let comp = world.extract_component::<BaseComponent>(&id).unwrap();
        
        comp.add_additional(DropTracked6 { id: i as usize });
        comp.add_additional(Additional1 { data: i as u64 * 2 });
        comp.add_additional(Additional2 { text: format!("entity_{}", i) });
        
        ids.push(id);
    }
    
    // Verify all additionals exist
    for (i, id) in ids.iter().enumerate() {
        let comp = world.extract_component::<BaseComponent>(id).unwrap();
        
        let tracked = comp.extract_additional::<DropTracked6>().unwrap();
        assert_eq!(tracked.id, i);
        
        let a1 = comp.extract_additional::<Additional1>().unwrap();
        assert_eq!(a1.data, i as u64 * 2);
        
        let a2 = comp.extract_additional::<Additional2>().unwrap();
        assert_eq!(a2.text, format!("entity_{}", i));
    }
    
    // Remove half
    for i in 0..500 {
        world.remove_entity(&ids[i]).unwrap();
    }
    
    // 500 dropped
    assert_eq!(DROP_COUNTER6.load(Ordering::SeqCst), 500);
    
    // Remove rest
    for i in 500..1000 {
        world.remove_entity(&ids[i]).unwrap();
    }
    
    // All dropped
    assert_eq!(DROP_COUNTER6.load(Ordering::SeqCst), 1000);
}

#[test]
fn test_additional_with_zst() {
    #[derive(Debug, Extractable)]
    struct ZeroSized;
    
    let world = World::new();
    let id = world.add_entity(BaseComponent { value: 42 });
    
    let comp = world.extract_component::<BaseComponent>(&id).unwrap();
    
    // Add ZST as additional
    comp.add_additional(ZeroSized);
    
    // Extract ZST
    let zst = comp.extract_additional::<ZeroSized>();
    assert!(zst.is_some());
    
    // Replace ZST
    comp.add_additional(ZeroSized);
    
    // Still extractable
    assert!(comp.extract_additional::<ZeroSized>().is_some());
}
