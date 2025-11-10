#![allow(dead_code)]

use structecs::*;

#[test]
fn sample_usage() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Entity {
        id: u32,
    }

    #[derive(Extractable, PartialEq, Debug)]
    #[extractable(entity)]
    struct NamedEntity {
        name: String,
        entity: Entity,
    }

    let named = NamedEntity {
        name: "Test".to_string(),
        entity: Entity { id: 42 },
    };
    let acquirable = Acquirable::new(named);
    let extracted_entity = acquirable.extract::<Entity>().unwrap();
    assert_eq!(*extracted_entity, Entity { id: 42 });
}

#[test]
fn test_weak_acquirable() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Entity {
        id: u32,
    }

    // Test upgrade when entity is alive
    let entity = Acquirable::new(Entity { id: 42 });
    let weak = entity.downgrade();

    assert!(weak.upgrade().is_some());
    assert_eq!(weak.upgrade().unwrap().id, 42);

    // Test upgrade after entity is dropped
    drop(entity);
    assert!(weak.upgrade().is_none());
}

#[test]
fn test_weak_acquirable_clone() {
    #[derive(Extractable, PartialEq, Debug)]
    struct Entity {
        id: u32,
    }

    let entity = Acquirable::new(Entity { id: 42 });
    let weak1 = entity.downgrade();
    let weak2 = weak1.clone();

    // Both weak references should work
    assert!(weak1.upgrade().is_some());
    assert!(weak2.upgrade().is_some());

    drop(entity);

    // Both should fail after entity is dropped
    assert!(weak1.upgrade().is_none());
    assert!(weak2.upgrade().is_none());
}

#[test]
fn test_ptr_eq() {
    #[derive(Extractable)]
    struct Entity {
        id: u32,
    }

    let entity1 = Acquirable::new(Entity { id: 42 });
    let entity2 = entity1.clone();
    let entity3 = Acquirable::new(Entity { id: 42 });

    // Same entity
    assert!(entity1.ptr_eq(&entity2));

    // Different entities
    assert!(!entity1.ptr_eq(&entity3));
}

#[cfg(debug_assertions)]
#[test]
fn test_reference_counting() {
    #[derive(Extractable)]
    struct Entity {
        id: u32,
    }

    let entity = Acquirable::new(Entity { id: 42 });
    assert_eq!(entity.strong_count(), 1);
    assert_eq!(entity.weak_count(), 0);

    let entity2 = entity.clone();
    assert_eq!(entity.strong_count(), 2);
    assert_eq!(entity.weak_count(), 0);

    let weak = entity.downgrade();
    assert_eq!(entity.strong_count(), 2);
    assert_eq!(entity.weak_count(), 1);

    let _weak2 = weak.clone();
    assert_eq!(entity.strong_count(), 2);
    assert_eq!(entity.weak_count(), 2);

    drop(weak);
    assert_eq!(entity.strong_count(), 2);
    assert_eq!(entity.weak_count(), 1);

    drop(entity2);
    assert_eq!(entity.strong_count(), 1);
    assert_eq!(entity.weak_count(), 1);
}

#[test]
fn test_circular_reference_prevention() {
    use std::cell::RefCell;

    #[derive(Extractable)]
    struct Node {
        id: u32,
        // Use weak reference to prevent circular reference
        parent: RefCell<Option<WeakAcquirable<Node>>>,
    }

    let parent = Acquirable::new(Node {
        id: 1,
        parent: RefCell::new(None),
    });

    let child = Acquirable::new(Node {
        id: 2,
        parent: RefCell::new(Some(parent.downgrade())),
    });

    // Parent is still alive
    assert!(child.parent.borrow().as_ref().unwrap().upgrade().is_some());

    drop(parent);

    // Parent is dropped, weak reference returns None
    assert!(child.parent.borrow().as_ref().unwrap().upgrade().is_none());
}
