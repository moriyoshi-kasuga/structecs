use structecs::*;
use std::mem::{align_of, size_of};

/// High alignment requirement (16 bytes)
#[repr(align(16))]
#[derive(Debug, Extractable)]
struct Aligned16 {
    value: u128,
}

/// Normal alignment (8 bytes)
#[derive(Debug, Extractable)]
struct Aligned8 {
    value: u64,
}

/// Low alignment (1 byte)
#[repr(align(1))]
#[derive(Debug, Extractable)]
struct Aligned1 {
    value: u8,
}

/// Zero-Sized Type
#[derive(Debug, Extractable)]
struct ZeroSized;

/// Mixed alignment struct with padding
#[derive(Debug, Extractable)]
struct MixedAlignment {
    a: u8,
    b: u64,
    c: u16,
}

/// Struct containing high-alignment field
#[derive(Debug, Extractable)]
struct ContainsAligned {
    x: u32,
    aligned: Aligned16,
    y: u32,
}

#[test]
fn test_high_alignment_entity() {
    let world = World::new();
    
    // Verify alignment requirements
    assert_eq!(align_of::<Aligned16>(), 16);
    assert_eq!(size_of::<Aligned16>(), 16);
    
    // Add entity with high alignment
    let id = world.add_entity(Aligned16 {
        value: 0x123456789ABCDEF0_123456789ABCDEF0,
    });
    
    // Extract and verify
    let component = world.extract_component::<Aligned16>(&id).unwrap();
    assert_eq!(component.value, 0x123456789ABCDEF0_123456789ABCDEF0);
    
    // Verify pointer alignment
    let ptr = &*component as *const Aligned16;
    assert_eq!(ptr as usize % 16, 0, "Pointer must be 16-byte aligned");
}

#[test]
fn test_mixed_alignment_components() {
    let world = World::new();
    
    // Create entity with mixed alignment fields
    let id = world.add_entity(MixedAlignment {
        a: 0x12,
        b: 0x123456789ABCDEF0,
        c: 0x3456,
    });
    
    // Extract and verify values
    let component = world.extract_component::<MixedAlignment>(&id).unwrap();
    assert_eq!(component.a, 0x12);
    assert_eq!(component.b, 0x123456789ABCDEF0);
    assert_eq!(component.c, 0x3456);
    
    // Verify that each field maintains proper alignment
    let ptr = &component.b as *const u64;
    assert_eq!(ptr as usize % 8, 0, "u64 field must be 8-byte aligned");
}

#[test]
fn test_zst_handling() {
    let world = World::new();
    
    // Verify ZST properties
    assert_eq!(size_of::<ZeroSized>(), 0);
    assert!(align_of::<ZeroSized>() >= 1);
    
    // Add ZST entity
    let id = world.add_entity(ZeroSized);
    
    // Extract ZST (should succeed even though it has no data)
    let component = world.extract_component::<ZeroSized>(&id);
    assert!(component.is_ok(), "ZST extraction should succeed");
    
    // Multiple ZST entities
    let mut ids = vec![];
    for _ in 0..100 {
        ids.push(world.add_entity(ZeroSized));
    }
    
    assert_eq!(world.entity_count(), 101);
    
    // Query ZST entities
    let count = world.query::<ZeroSized>().len();
    assert_eq!(count, 101);
}

#[test]
fn test_alignment_with_nested_high_alignment() {
    let world = World::new();
    
    // Entity containing high-alignment field
    let id = world.add_entity(ContainsAligned {
        x: 0x12345678,
        aligned: Aligned16 {
            value: 0xFEDCBA9876543210_FEDCBA9876543210,
        },
        y: 0x87654321,
    });
    
    // Extract and verify
    let component = world.extract_component::<ContainsAligned>(&id).unwrap();
    assert_eq!(component.x, 0x12345678);
    assert_eq!(component.aligned.value, 0xFEDCBA9876543210_FEDCBA9876543210);
    assert_eq!(component.y, 0x87654321);
    
    // Verify nested field alignment
    let aligned_ptr = &component.aligned as *const Aligned16;
    assert_eq!(aligned_ptr as usize % 16, 0, "Nested Aligned16 must be 16-byte aligned");
}

#[test]
fn test_alignment_with_additional() {
    let world = World::new();
    
    // Start with normal alignment entity
    let id = world.add_entity(Aligned8 { value: 0x123 });
    
    let component = world.extract_component::<Aligned8>(&id).unwrap();
    
    // Add high-alignment additional component
    component.add_additional(Aligned16 {
        value: 0xABCDEF0123456789_ABCDEF0123456789,
    });
    
    // Extract additional and verify alignment
    let additional = component.extract_additional::<Aligned16>().unwrap();
    assert_eq!(additional.value, 0xABCDEF0123456789_ABCDEF0123456789);
    
    let ptr = &*additional as *const Aligned16;
    assert_eq!(ptr as usize % 16, 0, "Additional component must maintain 16-byte alignment");
}

#[test]
fn test_multiple_alignments_in_additional() {
    let world = World::new();
    
    let id = world.add_entity(Aligned1 { value: 0x42 });
    let component = world.extract_component::<Aligned1>(&id).unwrap();
    
    // Add components with different alignments
    component.add_additional(Aligned8 { value: 0x111 });
    component.add_additional(Aligned16 { value: 0x222 });
    component.add_additional(Aligned1 { value: 0x33 });
    
    // Extract and verify all alignments
    let a8 = component.extract_additional::<Aligned8>().unwrap();
    let a16 = component.extract_additional::<Aligned16>().unwrap();
    let a1 = component.extract_additional::<Aligned1>().unwrap();
    
    assert_eq!(a8.value, 0x111);
    assert_eq!(a16.value, 0x222);
    assert_eq!(a1.value, 0x33);
    
    // Verify pointer alignments
    let ptr8 = &*a8 as *const Aligned8;
    let ptr16 = &*a16 as *const Aligned16;
    
    assert_eq!(ptr8 as usize % 8, 0, "Aligned8 must be 8-byte aligned");
    assert_eq!(ptr16 as usize % 16, 0, "Aligned16 must be 16-byte aligned");
}

#[test]
fn test_alignment_with_query() {
    let world = World::new();
    
    // Add multiple high-alignment entities
    for i in 0..100 {
        world.add_entity(Aligned16 { value: i as u128 });
    }
    
    // Query and verify all pointers are properly aligned
    let results = world.query::<Aligned16>();
    assert_eq!(results.len(), 100);
    
    for (_id, component) in results {
        let ptr = &*component as *const Aligned16;
        assert_eq!(ptr as usize % 16, 0, "Queried component must be 16-byte aligned");
    }
}

#[test]
fn test_zst_additional() {
    let world = World::new();
    
    let id = world.add_entity(Aligned8 { value: 0x123 });
    let component = world.extract_component::<Aligned8>(&id).unwrap();
    
    // Add ZST as additional
    component.add_additional(ZeroSized);
    
    // Extract ZST additional
    let zst = component.extract_additional::<ZeroSized>();
    assert!(zst.is_some(), "ZST additional should be extractable");
    
    // Add another ZST (should replace)
    component.add_additional(ZeroSized);
    
    let zst2 = component.extract_additional::<ZeroSized>();
    assert!(zst2.is_some(), "ZST additional should still be extractable");
}

#[test]
fn test_alignment_after_remove_and_readd() {
    let world = World::new();
    
    // Add and remove multiple times
    for iteration in 0..10 {
        let id = world.add_entity(Aligned16 { value: iteration as u128 });
        
        let component = world.extract_component::<Aligned16>(&id).unwrap();
        let ptr = &*component as *const Aligned16;
        assert_eq!(ptr as usize % 16, 0, "Must maintain alignment across add/remove cycles");
        
        world.remove_entity(&id).unwrap();
    }
}

#[test]
fn test_large_alignment_struct() {
    // Test with even larger alignment requirement
    #[repr(align(32))]
    #[derive(Debug, Extractable)]
    struct Aligned32 {
        data: [u64; 4],
    }
    
    let world = World::new();
    
    assert_eq!(align_of::<Aligned32>(), 32);
    
    let id = world.add_entity(Aligned32 {
        data: [1, 2, 3, 4],
    });
    
    let component = world.extract_component::<Aligned32>(&id).unwrap();
    assert_eq!(component.data, [1, 2, 3, 4]);
    
    let ptr = &*component as *const Aligned32;
    assert_eq!(ptr as usize % 32, 0, "Must respect 32-byte alignment");
}

#[test]
fn test_alignment_across_concurrent_operations() {
    use std::sync::Arc;
    use std::thread;
    
    let world = Arc::new(World::new());
    let mut handles = vec![];
    
    for thread_id in 0..10 {
        let world_clone = Arc::clone(&world);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                let id = world_clone.add_entity(Aligned16 {
                    value: (thread_id * 100 + i) as u128,
                });
                
                let component = world_clone.extract_component::<Aligned16>(&id).unwrap();
                let ptr = &*component as *const Aligned16;
                
                // Verify alignment even in concurrent scenario
                assert_eq!(ptr as usize % 16, 0, "Concurrent operations must maintain alignment");
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_pointer_provenance_with_offset() {
    // This test ensures that pointer arithmetic maintains provenance correctly
    let world = World::new();
    
    #[derive(Debug, Extractable)]
    struct MultiField {
        a: Aligned8,
        b: Aligned16,
        c: Aligned8,
    }
    
    let id = world.add_entity(MultiField {
        a: Aligned8 { value: 0x111 },
        b: Aligned16 { value: 0x222 },
        c: Aligned8 { value: 0x333 },
    });
    
    let component = world.extract_component::<MultiField>(&id).unwrap();
    
    // Verify all fields are accessible and correctly aligned
    assert_eq!(component.a.value, 0x111);
    assert_eq!(component.b.value, 0x222);
    assert_eq!(component.c.value, 0x333);
    
    let ptr_b = &component.b as *const Aligned16;
    assert_eq!(ptr_b as usize % 16, 0, "Nested field must maintain alignment");
}
